use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    agents,
    core::{
        ledger::Ledger,
        models::{
            AgentTask, LedgerEvent, LedgerEventType, OutcomeSpec, TestKind, TestManifest,
            TestSpec, TestStatus,
        },
        repository::Repository,
    },
    prompts::builder,
    runtime::filesystem::write_file,
};

// ── test add ──────────────────────────────────────────────────────────────────

pub struct AddArgs {
    pub id: String,
    pub feature_id: String,
    pub outcome_id: String,
    pub path: String,
    pub kind: crate::core::models::TestKind,
    pub purpose_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GeneratedTestsResponse {
    tests: Vec<GeneratedTest>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct GeneratedTest {
    feature_id: String,
    outcome_id: String,
    path: String,
    kind: String,
    #[serde(default)]
    purpose_refs: Vec<String>,
    content: String,
}

struct OutcomeGenerationContext {
    feature_id: String,
    outcome: OutcomeSpec,
}

struct PreparedTestGeneration {
    prompt: String,
    expected_paths: BTreeSet<(String, String, String)>,
    outcome_contexts: Vec<OutcomeGenerationContext>,
    scope_label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuggestedTestFile {
    pub feature_id: String,
    pub outcome_id: String,
    pub path: String,
    pub kind: String,
    pub purpose_refs: Vec<String>,
    pub content_line_count: usize,
    pub content_preview: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestSuggestionPreview {
    pub agent: String,
    pub scope_label: String,
    pub expected_paths: Vec<String>,
    pub missing_expected_paths: Vec<String>,
    pub unexpected_paths: Vec<String>,
    pub exact_path_match: bool,
    pub suggestions: Vec<SuggestedTestFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestGenerationRunSummary {
    pub agent: String,
    pub scope_label: String,
    pub generated_count: usize,
}

pub fn add(repo: &Repository, args: AddArgs) -> Result<()> {
    // Verify feature and outcome exist
    repo.load_feature(&args.feature_id)?;
    repo.load_outcome(&args.feature_id, &args.outcome_id)?;

    let mut manifest = repo.load_manifest()?;

    add_to_manifest(&mut manifest, args, TestStatus::Planned)?;
    repo.save_manifest(&manifest)?;

    let test = manifest.tests.last().context("manifest missing inserted test")?;

    let event = LedgerEvent::new(LedgerEventType::TestAdded)
        .with_feature(&test.feature_id)
        .with_outcome(&test.outcome_id)
        .with_message(format!("test '{}' added", test.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    println!("✓ Test '{}' added to manifest.", test.id);
    println!("  Path:    {}", test.path);
    println!(
        "  Status:  planned — update to 'written' once the test file exists"
    );
    Ok(())
}

pub fn generate(repo: &Repository, agent_override: Option<&str>) -> Result<()> {
    generate_scoped(repo, None, None, agent_override)?;
    Ok(())
}

pub fn generate_scoped(
    repo: &Repository,
    feature_id: Option<&str>,
    outcome_id: Option<&str>,
    agent_override: Option<&str>,
) -> Result<TestGenerationRunSummary> {
    let prepared = prepare_test_generation(repo, feature_id, outcome_id)?;
    let mut manifest = repo.load_manifest()?;
    let agent_name = agent_override.unwrap_or("copilot");

    println!("▶ Running agent '{agent_name}' to generate required tests…");
    println!("{}", "─".repeat(60));
    let agent_output = run_test_generation_agent(repo, &prepared, agent_override)?;
    let agent_name = agent_output.agent_name.as_str();

    let run_event = LedgerEvent::new(LedgerEventType::TestGenerationRun)
        .with_agent(agent_name)
        .with_success(agent_output.result.success);
    Ledger::append(&repo.ledger_path(), &run_event)?;

    if !agent_output.result.success {
        println!("stdout:\n{}", agent_output.result.stdout);
        if !agent_output.result.stderr.is_empty() {
            eprintln!("stderr:\n{}", agent_output.result.stderr);
        }
        bail!(
            "test generation agent exited with non-zero status ({})",
            agent_output.result.exit_code.unwrap_or(-1)
        );
    }

    let response = parse_generated_tests(&agent_output.result.stdout)?;
    let actual_paths = generated_test_paths(&response);

    if actual_paths != prepared.expected_paths {
        bail!("generated tests did not match outcome.required_tests declarations");
    }

    let mut generated_count = 0usize;
    for generated in response.tests {
        let outcome = prepared.outcome_contexts
            .iter()
            .find(|context| {
                context.feature_id == generated.feature_id
                    && context.outcome.id == generated.outcome_id
            })
            .map(|context| &context.outcome)
            .with_context(|| {
                format!(
                    "generated unknown outcome '{}:{}'",
                    generated.feature_id, generated.outcome_id
                )
            })?;

        if !outcome.required_tests.iter().any(|path| path == &generated.path) {
            bail!(
                "generated test path '{}' is not declared in outcome.required_tests for '{}:{}'",
                generated.path,
                generated.feature_id,
                generated.outcome_id
            );
        }

        let kind = parse_generated_test_kind(&generated.kind)?;
        let id = generate_test_id(&generated.feature_id, &generated.outcome_id, &generated.path);

        write_file(&repo.root.join(&generated.path), &generated.content)?;

        add_to_manifest(
            &mut manifest,
            AddArgs {
                id: id.clone(),
                feature_id: generated.feature_id.clone(),
                outcome_id: generated.outcome_id.clone(),
                path: generated.path.clone(),
                kind,
                purpose_refs: generated.purpose_refs.clone(),
            },
            TestStatus::Written,
        )?;

        let event = LedgerEvent::new(LedgerEventType::TestAdded)
            .with_feature(&generated.feature_id)
            .with_outcome(&generated.outcome_id)
            .with_message(format!("test '{}' generated", id));
        Ledger::append(&repo.ledger_path(), &event)?;
        generated_count = generated_count.saturating_add(1);
    }

    repo.save_manifest(&manifest)?;

    println!("Generated {generated_count} test file(s) and updated the manifest.");
    println!("{}", "─".repeat(60));
    Ok(TestGenerationRunSummary {
        agent: agent_name.to_string(),
        scope_label: prepared.scope_label,
        generated_count,
    })
}

pub fn suggest(
    repo: &Repository,
    feature_id: Option<&str>,
    outcome_id: Option<&str>,
    agent_override: Option<&str>,
) -> Result<()> {
    let preview = preview(repo, feature_id, outcome_id, agent_override)?;

    println!(
        "Previewed {} suggested test file(s) with agent '{}' for {}.",
        preview.suggestions.len(),
        preview.agent,
        preview.scope_label
    );
    println!(
        "Path match: {}",
        if preview.exact_path_match {
            "exact"
        } else {
            "mismatch"
        }
    );

    if !preview.suggestions.is_empty() {
        println!("\nSuggested files:");
        for suggestion in &preview.suggestions {
            println!(
                "  - {} [{}] {} line(s)",
                suggestion.path, suggestion.kind, suggestion.content_line_count
            );
        }
    }

    if !preview.missing_expected_paths.is_empty() {
        println!("\nMissing expected paths:");
        for path in &preview.missing_expected_paths {
            println!("  - {path}");
        }
    }

    if !preview.unexpected_paths.is_empty() {
        println!("\nUnexpected generated paths:");
        for path in &preview.unexpected_paths {
            println!("  - {path}");
        }
    }

    println!("\nPreview only: no files were written and the manifest was unchanged.");
    Ok(())
}

pub fn preview(
    repo: &Repository,
    feature_id: Option<&str>,
    outcome_id: Option<&str>,
    agent_override: Option<&str>,
) -> Result<TestSuggestionPreview> {
    let prepared = prepare_test_generation(repo, feature_id, outcome_id)?;
    let agent_output = run_test_generation_agent(repo, &prepared, agent_override)?;

    if !agent_output.result.success {
        bail!(
            "test suggestion agent exited with non-zero status ({})\nstdout:\n{}\nstderr:\n{}",
            agent_output.result.exit_code.unwrap_or(-1),
            agent_output.result.stdout,
            agent_output.result.stderr
        );
    }

    let response = parse_generated_tests(&agent_output.result.stdout)?;
    let actual_paths = generated_test_paths(&response);
    let expected_paths = prepared
        .expected_paths
        .iter()
        .map(|(_, _, path)| path.clone())
        .collect::<Vec<_>>();
    let missing_expected_paths = prepared
        .expected_paths
        .difference(&actual_paths)
        .map(format_generated_path_tuple)
        .collect::<Vec<_>>();
    let unexpected_paths = actual_paths
        .difference(&prepared.expected_paths)
        .map(format_generated_path_tuple)
        .collect::<Vec<_>>();
    let exact_path_match = missing_expected_paths.is_empty() && unexpected_paths.is_empty();
    let suggestions = response
        .tests
        .into_iter()
        .map(|test| SuggestedTestFile {
            content_line_count: test.content.lines().count(),
            content_preview: summarize_content_preview(&test.content),
            feature_id: test.feature_id,
            outcome_id: test.outcome_id,
            path: test.path,
            kind: test.kind,
            purpose_refs: test.purpose_refs,
        })
        .collect::<Vec<_>>();

    Ok(TestSuggestionPreview {
        agent: agent_output.agent_name,
        scope_label: prepared.scope_label,
        expected_paths,
        missing_expected_paths,
        unexpected_paths: unexpected_paths.clone(),
        exact_path_match,
        suggestions,
    })
}

fn prepare_test_generation(
    repo: &Repository,
    feature_filter: Option<&str>,
    outcome_filter: Option<&str>,
) -> Result<PreparedTestGeneration> {
    if outcome_filter.is_some() && feature_filter.is_none() {
        bail!("outcome filter requires a feature filter");
    }

    let config = repo.load_config()?;
    let manifest = repo.load_manifest()?;
    let features = match feature_filter {
        Some(feature_id) => vec![repo.load_feature(feature_id)?],
        None => repo.list_features()?,
    };

    if features.is_empty() {
        bail!("no features found — define features and outcomes before generating tests");
    }

    let mut prompt_features = Vec::new();
    let mut outcome_contexts = Vec::new();
    let mut expected_paths = BTreeSet::new();

    for feature in features {
        let mut outcomes = repo.list_outcomes(&feature.id)?;
        if let Some(selected_outcome_id) = outcome_filter {
            repo.load_outcome(&feature.id, selected_outcome_id)?;
            outcomes.retain(|outcome| outcome.id == selected_outcome_id);
        }

        outcomes.retain(|outcome| !outcome.required_tests.is_empty());

        for outcome in &outcomes {
            for path in &outcome.required_tests {
                expected_paths.insert((feature.id.clone(), outcome.id.clone(), path.clone()));
            }

            outcome_contexts.push(OutcomeGenerationContext {
                feature_id: feature.id.clone(),
                outcome: outcome.clone(),
            });
        }

        if !outcomes.is_empty() {
            prompt_features.push((feature, outcomes));
        }
    }

    if expected_paths.is_empty() {
        bail!("no outcome.required_tests entries found — add required test paths to your outcomes first");
    }

    let prompt = builder::build_test_generation_prompt(&config, &prompt_features, &manifest)?;
    let scope_label = match (feature_filter, outcome_filter) {
        (Some(feature_id), Some(outcome_id)) => format!("{feature_id}:{outcome_id}"),
        (Some(feature_id), None) => format!("feature '{feature_id}'"),
        _ => "the project".to_string(),
    };

    Ok(PreparedTestGeneration {
        prompt,
        expected_paths,
        outcome_contexts,
        scope_label,
    })
}

struct GenerationAgentOutput {
    agent_name: String,
    result: crate::core::models::AgentRunResult,
}

fn run_test_generation_agent(
    repo: &Repository,
    prepared: &PreparedTestGeneration,
    agent_override: Option<&str>,
) -> Result<GenerationAgentOutput> {
    let agent_name = agent_override.unwrap_or("copilot").to_string();
    let task = AgentTask {
        feature_id: "project".to_string(),
        outcome_id: "all-required-tests".to_string(),
        agent: agent_name.clone(),
        prompt: prepared.prompt.clone(),
        allowed_paths: prepared
            .expected_paths
            .iter()
            .map(|(_, _, path)| path.clone())
            .collect(),
        forbidden_paths: Vec::new(),
    };
    let result = agents::run_task(&agent_name, &task, Some(&repo.root))?;

    Ok(GenerationAgentOutput { agent_name, result })
}

fn generated_test_paths(response: &GeneratedTestsResponse) -> BTreeSet<(String, String, String)> {
    response
        .tests
        .iter()
        .map(|test| (test.feature_id.clone(), test.outcome_id.clone(), test.path.clone()))
        .collect()
}

fn format_generated_path_tuple(value: &(String, String, String)) -> String {
    format!("{}:{}:{}", value.0, value.1, value.2)
}

fn summarize_content_preview(content: &str) -> String {
    let first_non_empty = content
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();

    if first_non_empty.chars().count() <= 80 {
        first_non_empty.to_string()
    } else {
        first_non_empty.chars().take(80).collect::<String>() + "…"
    }
}

// ── test list ─────────────────────────────────────────────────────────────────

pub fn list(repo: &Repository, feature_id: Option<&str>, outcome_id: Option<&str>) -> Result<()> {
    let manifest = repo.load_manifest()?;
    let tests: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| {
            feature_id.map_or(true, |f| t.feature_id == f)
                && outcome_id.map_or(true, |o| t.outcome_id == o)
        })
        .collect();

    if tests.is_empty() {
        println!("No tests found.");
        return Ok(());
    }

    println!(
        "{:<35} {:<20} {:<20} {:<12} {}",
        "ID", "FEATURE", "OUTCOME", "KIND", "STATUS"
    );
    println!("{}", "─".repeat(100));
    for t in &tests {
        let kind = format!("{:?}", t.kind).to_lowercase();
        let status = format!("{:?}", t.status).to_lowercase();
        println!(
            "{:<35} {:<20} {:<20} {:<12} {}",
            t.id, t.feature_id, t.outcome_id, kind, status
        );
    }
    Ok(())
}

// ── test set-status ───────────────────────────────────────────────────────────

pub fn set_status(
    repo: &Repository,
    test_id: &str,
    status: crate::core::models::TestStatus,
) -> Result<()> {
    let mut manifest = repo.load_manifest()?;

    let test = manifest
        .tests
        .iter_mut()
        .find(|t| t.id == test_id)
        .ok_or_else(|| anyhow::anyhow!("test '{}' not found in manifest", test_id))?;

    test.status = status.clone();
    repo.save_manifest(&manifest)?;

    let status_str = format!("{:?}", status).to_lowercase();
    println!("✓ Test '{test_id}' status updated to '{status_str}'.");
    Ok(())
}

fn add_to_manifest(manifest: &mut TestManifest, args: AddArgs, status: TestStatus) -> Result<()> {
    if manifest.tests.iter().any(|t| t.id == args.id) {
        bail!("test '{}' already exists in the manifest", args.id);
    }

    if let Some(existing) = manifest.tests.iter_mut().find(|t| {
        t.feature_id == args.feature_id && t.outcome_id == args.outcome_id && t.path == args.path
    }) {
        existing.kind = args.kind;
        existing.purpose_refs = args.purpose_refs;
        existing.status = status;
        return Ok(());
    }

    manifest.tests.push(TestSpec {
        id: args.id,
        feature_id: args.feature_id,
        outcome_id: args.outcome_id,
        path: args.path,
        purpose_refs: args.purpose_refs,
        kind: args.kind,
        status,
    });

    Ok(())
}

fn parse_generated_tests(output: &str) -> Result<GeneratedTestsResponse> {
    let cleaned = strip_ansi_escape_sequences(output);
    let trimmed = cleaned.trim();

    if let Ok(response) = serde_json::from_str(trimmed) {
        return Ok(response);
    }

    let mut last_error = None;
    for candidate in extract_json_object_candidates(trimmed) {
        match serde_json::from_str(&candidate) {
            Ok(response) => return Ok(response),
            Err(error) => last_error = Some(error),
        }
    }

    match last_error {
        Some(error) => Err(error).with_context(|| "parsing generated test response as JSON"),
        None => bail!("parsing generated test response as JSON: no JSON object found in agent output"),
    }
}

fn strip_ansi_escape_sequences(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && matches!(chars.peek(), Some('[')) {
            chars.next();
            while let Some(next) = chars.next() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
            continue;
        }

        cleaned.push(ch);
    }

    cleaned
}

fn extract_json_object_candidates(value: &str) -> Vec<String> {
    let mut candidates = Vec::new();

    for (start, ch) in value.char_indices() {
        if ch != '{' {
            continue;
        }

        if let Some(end) = find_balanced_json_object_end(&value[start..]) {
            candidates.push(value[start..start + end].to_string());
        }
    }

    candidates
}

fn find_balanced_json_object_end(value: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth = depth.saturating_add(1),
            '}' => {
                if depth == 0 {
                    return None;
                }

                depth -= 1;
                if depth == 0 {
                    return Some(index + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}

fn parse_generated_test_kind(value: &str) -> Result<TestKind> {
    match value.to_lowercase().as_str() {
        "unit" => Ok(TestKind::Unit),
        "integration" => Ok(TestKind::Integration),
        "e2e" => Ok(TestKind::E2e),
        other => bail!("unknown generated test kind '{other}'"),
    }
}

fn generate_test_id(feature_id: &str, outcome_id: &str, path: &str) -> String {
    format!("{feature_id}-{outcome_id}-{path}")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::parse_generated_tests;

    #[test]
    fn parse_generated_tests_accepts_fenced_json_with_leading_text() {
        let output = r##"
Here are the required tests.

```json
{
    "tests": [
        {
            "feature_id": "auth",
            "outcome_id": "outcome-1",
            "path": "tests/auth/validate.rs",
            "kind": "unit",
            "purpose_refs": ["goal:Validate credentials."],
            "content": "#[test]\nfn validates_credentials() {\n    assert!(true);\n}\n"
        }
    ]
}
```
"##;

        let parsed = parse_generated_tests(output).expect("expected fenced JSON to parse");
        assert_eq!(parsed.tests.len(), 1);
        assert_eq!(parsed.tests[0].path, "tests/auth/validate.rs");
    }

    #[test]
    fn parse_generated_tests_accepts_ansi_wrapped_json() {
        let output = "\u{1b}[32m{\n  \"tests\": [{\n    \"feature_id\": \"auth\",\n    \"outcome_id\": \"outcome-1\",\n    \"path\": \"tests/auth/validate.rs\",\n    \"kind\": \"unit\",\n    \"purpose_refs\": [],\n    \"content\": \"#[test]\\nfn validates_credentials() {\\n    assert!(true);\\n}\\n\"\n  }]\n}\u{1b}[0m";

        let parsed = parse_generated_tests(output).expect("expected ANSI-wrapped JSON to parse");
        assert_eq!(parsed.tests.len(), 1);
        assert_eq!(parsed.tests[0].feature_id, "auth");
    }

    #[test]
    fn parse_generated_tests_reports_missing_json() {
        let error = match parse_generated_tests("No JSON here") {
            Ok(_) => panic!("expected parse to fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("no JSON object found"));
    }
}
