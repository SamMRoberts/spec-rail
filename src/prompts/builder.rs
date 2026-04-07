use crate::core::models::{FeatureSpec, PhaseSpec, TestManifest};

/// Build the implementation prompt sent to an AI coding agent.
///
/// The prompt communicates the feature purpose, phase goal, scope constraints,
/// and the tests that must pass — in a structured, unambiguous format.
pub fn build_implementation_prompt(
    feature: &FeatureSpec,
    phase: &PhaseSpec,
    manifest: &TestManifest,
) -> String {
    let tests_for_phase: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| t.phase_id == phase.id && t.feature_id == feature.id)
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

    // Phase context
    prompt.push_str("## Current Phase\n");
    prompt.push_str(&format!("ID: {}\n", phase.id));
    prompt.push_str(&format!("Title: {}\n", phase.title));
    prompt.push_str(&format!("Goal: {}\n", phase.goal));
    prompt.push_str(&format!("Order: {}\n\n", phase.order));

    // Scope constraints
    if !phase.allowed_paths.is_empty() {
        prompt.push_str("### Allowed Paths (implement only here)\n");
        for p in &phase.allowed_paths {
            prompt.push_str(&format!("- {p}\n"));
        }
        prompt.push('\n');
    }

    if !phase.forbidden_paths.is_empty() {
        prompt.push_str("### Forbidden Paths (do NOT touch)\n");
        for p in &phase.forbidden_paths {
            prompt.push_str(&format!("- {p}\n"));
        }
        prompt.push('\n');
    }

    // Required tests
    if !tests_for_phase.is_empty() {
        prompt.push_str("## Tests to Pass\n");
        prompt.push_str(
            "The following tests already exist. Your implementation must make them pass.\n\n",
        );
        for t in &tests_for_phase {
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
            "## Tests\nNo tests are registered for this phase yet. \
             Add tests with `specrail test add` before running the agent.\n\n",
        );
    }

    prompt.push_str("## Instructions\n");
    prompt.push_str(
        "1. Implement ONLY the current phase. Do not implement future phases.\n\
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
