use crate::core::{
    config::ProjectConfig,
    models::{FeatureSpec, OutcomeSpec, TestManifest},
};

/// Build the implementation prompt sent to an AI coding agent.
///
/// The prompt communicates the feature purpose, outcome goal, scope constraints,
/// and the tests that must pass — in a structured, unambiguous format.
pub fn build_implementation_prompt(
    feature: &FeatureSpec,
    outcome: &OutcomeSpec,
    manifest: &TestManifest,
) -> String {
    let tests_for_outcome: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| t.outcome_id == outcome.id && t.feature_id == feature.id)
        .collect();

    let mut prompt = String::new();

    prompt.push_str("# specrail — Implementation Task\n\n");

    // Feature context
    prompt.push_str("## Feature\n");
    prompt.push_str(&format!("ID: {}\n", feature.id));
    prompt.push_str(&format!("Title: {}\n", feature.title));
    prompt.push_str(&format!("Purpose: {}\n\n", feature.purpose));

    if !feature.outcomes.is_empty() {
        prompt.push_str("### Outcomes\n");
        for o in &feature.outcomes {
            prompt.push_str(&format!("- {o}\n"));
        }
        prompt.push('\n');
    }

    if !feature.constraints.is_empty() {
        prompt.push_str("### Constraints\n");
        for c in &feature.constraints {
            prompt.push_str(&format!("- {c}\n"));
        }
        prompt.push('\n');
    }

    if !feature.non_goals.is_empty() {
        prompt.push_str("### Non-goals\n");
        for ng in &feature.non_goals {
            prompt.push_str(&format!("- {ng}\n"));
        }
        prompt.push('\n');
    }

    // Outcome context
    prompt.push_str("## Current Outcome\n");
    prompt.push_str(&format!("ID: {}\n", outcome.id));
    prompt.push_str(&format!("Title: {}\n", outcome.title));
    prompt.push_str(&format!("Goal: {}\n", outcome.goal));
    prompt.push_str(&format!("Order: {}\n\n", outcome.order));

    // Scope constraints
    if !outcome.allowed_paths.is_empty() {
        prompt.push_str("### Allowed Paths (implement only here)\n");
        for p in &outcome.allowed_paths {
            prompt.push_str(&format!("- {p}\n"));
        }
        prompt.push('\n');
    }

    if !outcome.forbidden_paths.is_empty() {
        prompt.push_str("### Forbidden Paths (do NOT touch)\n");
        for p in &outcome.forbidden_paths {
            prompt.push_str(&format!("- {p}\n"));
        }
        prompt.push('\n');
    }

    // Required tests
    if !tests_for_outcome.is_empty() {
        prompt.push_str("## Tests to Pass\n");
        prompt.push_str(
            "The following tests already exist. Your implementation must make them pass.\n\n",
        );
        for t in &tests_for_outcome {
            prompt.push_str(&format!("- `{}` ({})\n", t.path, format_kind(&t.kind)));
            if !t.purpose_refs.is_empty() {
                for r in &t.purpose_refs {
                    prompt.push_str(&format!("  purpose: {r}\n"));
                }
            }
        }
        prompt.push('\n');
    } else {
        prompt.push_str(
            "## Tests\nNo tests are registered for this outcome yet. \
             Add tests with `specrail test add` before running the agent.\n\n",
        );
    }

    prompt.push_str("## Instructions\n");
    prompt.push_str(
        "1. Implement ONLY the current outcome. Do not implement future outcomes.\n\
         2. Stay within the allowed paths. Do not touch forbidden paths.\n\
         3. Make all listed tests pass.\n\
         4. Do not remove or modify existing tests.\n\
         5. Keep the implementation minimal and purposeful.\n",
    );

    prompt
}

fn format_kind(kind: &crate::core::models::TestKind) -> &'static str {
    use crate::core::models::TestKind;
    match kind {
        TestKind::Unit => "unit",
        TestKind::Integration => "integration",
        TestKind::E2e => "e2e",
    }
}

pub fn build_test_generation_prompt(
    config: &ProjectConfig,
    features: &[(FeatureSpec, Vec<OutcomeSpec>)],
    manifest: &TestManifest,
    allow_path_discovery: bool,
) -> anyhow::Result<String> {
    let mut prompt = String::new();

    prompt.push_str("# specrail - Test Generation Task\n\n");
    prompt.push_str("Generate the required test files declared in outcome YAML.\n");
    if allow_path_discovery {
        prompt.push_str("If the scoped outcome has no required_tests yet, choose exactly one canonical test path for it and include that path in the response.\n");
    }
    prompt.push_str("Return JSON only. Do not wrap the JSON in markdown fences.\n\n");

    prompt.push_str("## Response format\n");
    prompt.push_str("Return a JSON object with this shape:\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"tests\": [\n");
    prompt.push_str("    {\n");
    prompt.push_str("      \"feature_id\": \"auth\",\n");
    prompt.push_str("      \"outcome_id\": \"outcome-1\",\n");
    prompt.push_str("      \"id\": \"validates_credentials\",\n");
    prompt.push_str("      \"path\": \"tests/auth/validate.rs\",\n");
    prompt.push_str("      \"kind\": \"unit\",\n");
    prompt.push_str("      \"purpose_refs\": [\"goal:Validate credentials\"],\n");
    prompt.push_str("      \"content\": \"full file contents here\"\n");
    prompt.push_str("    }\n");
    prompt.push_str("  ]\n");
    prompt.push_str("}\n\n");

    prompt.push_str("## Rules\n");
    prompt.push_str("1. Generate exactly one test object for each required test declared in outcome.required_tests and outcome.required_test_files.\n");
    if allow_path_discovery {
        prompt.push_str("2. If the scoped outcome has no complete required test metadata yet, generate exactly one focused test object for that outcome and choose a canonical test id and path for it.\n");
        prompt.push_str("3. Do not invent extra test ids or paths beyond the declared required test metadata, except for that single bootstrap test when metadata is incomplete.\n");
        prompt.push_str("4. Use the declared feature_id and outcome_id for each generated test.\n");
        prompt.push_str("5. Set the generated test object's id to the required manifest test id or method name for that test.\n");
        prompt.push_str("6. The content must be a complete file that can be written directly to disk.\n");
        prompt.push_str("7. Prefer minimal, focused tests that align to the feature purpose and outcome goal.\n");
        prompt.push_str("8. If a required test path already appears in the manifest, regenerate it with updated content but keep the same path and id.\n\n");
    } else {
        prompt.push_str("2. Do not invent extra test ids or paths.\n");
        prompt.push_str("3. Use the declared feature_id and outcome_id for each generated test.\n");
        prompt.push_str("4. Set the generated test object's id to the declared required test id or method name for that test.\n");
        prompt.push_str("5. The content must be a complete file that can be written directly to disk.\n");
        prompt.push_str("6. Prefer minimal, focused tests that align to the feature purpose and outcome goal.\n");
        prompt.push_str("7. If a required test path already appears in the manifest, regenerate it with updated content but keep the same path and id.\n\n");
    }

    prompt.push_str("## Project YAML\n```yaml\n");
    prompt.push_str(&serde_yaml::to_string(config)?);
    prompt.push_str("```\n\n");

    for (feature, outcomes) in features {
        prompt.push_str("## Feature YAML\n```yaml\n");
        prompt.push_str(&serde_yaml::to_string(feature)?);
        prompt.push_str("```\n\n");

        for outcome in outcomes {
            prompt.push_str("### Outcome YAML\n```yaml\n");
            prompt.push_str(&serde_yaml::to_string(outcome)?);
            prompt.push_str("```\n\n");
        }
    }

    prompt.push_str("## Existing Manifest YAML\n```yaml\n");
    prompt.push_str(&serde_yaml::to_string(manifest)?);
    prompt.push_str("```\n");

    Ok(prompt)
}
