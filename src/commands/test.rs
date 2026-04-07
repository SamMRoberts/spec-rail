use std::collections::BTreeSet;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::{
    agents,
    core::{
        ledger::Ledger,
        models::{
            AgentTask, LedgerEvent, LedgerEventType, PhaseSpec, TestKind, TestManifest,
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
    pub phase_id: String,
    pub path: String,
    pub kind: crate::core::models::TestKind,
    pub purpose_refs: Vec<String>,
}

#[derive(Deserialize)]
struct GeneratedTestsResponse {
    tests: Vec<GeneratedTest>,
}

#[derive(Deserialize)]
struct GeneratedTest {
    feature_id: String,
    phase_id: String,
    path: String,
    kind: String,
    #[serde(default)]
    purpose_refs: Vec<String>,
    content: String,
}

struct PhaseGenerationContext {
    feature_id: String,
    phase: PhaseSpec,
}

pub fn add(repo: &Repository, args: AddArgs) -> Result<()> {
    // Verify feature and phase exist
    repo.load_feature(&args.feature_id)?;
    repo.load_phase(&args.feature_id, &args.phase_id)?;

    let mut manifest = repo.load_manifest()?;

    add_to_manifest(&mut manifest, args, TestStatus::Planned)?;
    repo.save_manifest(&manifest)?;

    let test = manifest.tests.last().context("manifest missing inserted test")?;

    let event = LedgerEvent::new(LedgerEventType::TestAdded)
        .with_feature(&test.feature_id)
        .with_phase(&test.phase_id)
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
    let config = repo.load_config()?;
    let mut manifest = repo.load_manifest()?;
    let features = repo.list_features()?;

    if features.is_empty() {
        bail!("no features found — define features and phases before generating tests");
    }

    let mut prompt_features = Vec::new();
    let mut phase_contexts = Vec::new();
    let mut expected_paths = BTreeSet::new();

    for feature in features {
        let phases = repo.list_phases(&feature.id)?;
        for phase in &phases {
            if phase.required_tests.is_empty() {
                continue;
            }

            for path in &phase.required_tests {
                expected_paths.insert((feature.id.clone(), phase.id.clone(), path.clone()));
            }

            phase_contexts.push(PhaseGenerationContext {
                feature_id: feature.id.clone(),
                phase: phase.clone(),
            });
        }
        prompt_features.push((feature, phases));
    }

    if expected_paths.is_empty() {
        bail!("no phase.required_tests entries found — add required test paths to your phases first");
    }

    let prompt = builder::build_test_generation_prompt(&config, &prompt_features, &manifest)?;
    let agent_name = agent_override.unwrap_or("copilot");

    println!("▶ Running agent '{agent_name}' to generate required tests…");
    println!("{}", "─".repeat(60));

    let task = AgentTask {
        feature_id: "project".to_string(),
        phase_id: "all-required-tests".to_string(),
        agent: agent_name.to_string(),
        prompt,
        allowed_paths: expected_paths.iter().map(|(_, _, path)| path.clone()).collect(),
        forbidden_paths: Vec::new(),
    };

    let result = agents::run_task(agent_name, &task, Some(&repo.root))?;

    let run_event = LedgerEvent::new(LedgerEventType::TestGenerationRun)
        .with_agent(agent_name)
        .with_success(result.success);
    Ledger::append(&repo.ledger_path(), &run_event)?;

    if !result.success {
        println!("stdout:\n{}", result.stdout);
        if !result.stderr.is_empty() {
            eprintln!("stderr:\n{}", result.stderr);
        }
        bail!(
            "test generation agent exited with non-zero status ({})",
            result.exit_code.unwrap_or(-1)
        );
    }

    let response = parse_generated_tests(&result.stdout)?;
    let actual_paths: BTreeSet<_> = response
        .tests
        .iter()
        .map(|test| (test.feature_id.clone(), test.phase_id.clone(), test.path.clone()))
        .collect();

    if actual_paths != expected_paths {
        bail!("generated tests did not match phase.required_tests declarations");
    }

    let mut generated_count = 0usize;
    for generated in response.tests {
        let phase = phase_contexts
            .iter()
            .find(|context| {
                context.feature_id == generated.feature_id
                    && context.phase.id == generated.phase_id
            })
            .map(|context| &context.phase)
            .with_context(|| {
                format!(
                    "generated unknown phase '{}:{}'",
                    generated.feature_id, generated.phase_id
                )
            })?;

        if !phase.required_tests.iter().any(|path| path == &generated.path) {
            bail!(
                "generated test path '{}' is not declared in phase.required_tests for '{}:{}'",
                generated.path,
                generated.feature_id,
                generated.phase_id
            );
        }

        let kind = parse_generated_test_kind(&generated.kind)?;
        let id = generate_test_id(&generated.feature_id, &generated.phase_id, &generated.path);

        write_file(&repo.root.join(&generated.path), &generated.content)?;

        add_to_manifest(
            &mut manifest,
            AddArgs {
                id: id.clone(),
                feature_id: generated.feature_id.clone(),
                phase_id: generated.phase_id.clone(),
                path: generated.path.clone(),
                kind,
                purpose_refs: generated.purpose_refs.clone(),
            },
            TestStatus::Written,
        )?;

        let event = LedgerEvent::new(LedgerEventType::TestAdded)
            .with_feature(&generated.feature_id)
            .with_phase(&generated.phase_id)
            .with_message(format!("test '{}' generated", id));
        Ledger::append(&repo.ledger_path(), &event)?;
        generated_count = generated_count.saturating_add(1);
    }

    repo.save_manifest(&manifest)?;

    println!("Generated {generated_count} test file(s) and updated the manifest.");
    println!("{}", "─".repeat(60));
    Ok(())
}

// ── test list ─────────────────────────────────────────────────────────────────

pub fn list(repo: &Repository, feature_id: Option<&str>, phase_id: Option<&str>) -> Result<()> {
    let manifest = repo.load_manifest()?;
    let tests: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| {
            feature_id.map_or(true, |f| t.feature_id == f)
                && phase_id.map_or(true, |p| t.phase_id == p)
        })
        .collect();

    if tests.is_empty() {
        println!("No tests found.");
        return Ok(());
    }

    println!(
        "{:<35} {:<20} {:<20} {:<12} {}",
        "ID", "FEATURE", "PHASE", "KIND", "STATUS"
    );
    println!("{}", "─".repeat(100));
    for t in &tests {
        let kind = format!("{:?}", t.kind).to_lowercase();
        let status = format!("{:?}", t.status).to_lowercase();
        println!(
            "{:<35} {:<20} {:<20} {:<12} {}",
            t.id, t.feature_id, t.phase_id, kind, status
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
        t.feature_id == args.feature_id && t.phase_id == args.phase_id && t.path == args.path
    }) {
        existing.kind = args.kind;
        existing.purpose_refs = args.purpose_refs;
        existing.status = status;
        return Ok(());
    }

    manifest.tests.push(TestSpec {
        id: args.id,
        feature_id: args.feature_id,
        phase_id: args.phase_id,
        path: args.path,
        purpose_refs: args.purpose_refs,
        kind: args.kind,
        status,
    });

    Ok(())
}

fn parse_generated_tests(output: &str) -> Result<GeneratedTestsResponse> {
    let trimmed = output.trim();
    let json = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    serde_json::from_str(json).with_context(|| "parsing generated test response as JSON")
}

fn parse_generated_test_kind(value: &str) -> Result<TestKind> {
    match value.to_lowercase().as_str() {
        "unit" => Ok(TestKind::Unit),
        "integration" => Ok(TestKind::Integration),
        "e2e" => Ok(TestKind::E2e),
        other => bail!("unknown generated test kind '{other}'"),
    }
}

fn generate_test_id(feature_id: &str, phase_id: &str, path: &str) -> String {
    format!("{feature_id}-{phase_id}-{path}")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}
