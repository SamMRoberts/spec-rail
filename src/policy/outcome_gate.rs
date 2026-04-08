use anyhow::Result;

use crate::core::models::{OutcomeSpec, OutcomeStatus, TestManifest, TestStatus};

/// Check whether all policy gates pass for entering the implementation step of
/// the given outcome.
///
/// Rules enforced:
///
/// 1. The outcome must be in `pending` or `active` status.
/// 2. At least one test must be registered for the outcome in the manifest.
/// 3. All registered tests must be in `written`, `passing`, or `failing`
///    status — **not** `planned`.  Tests must exist before implementation.
///
/// Returns `Ok(())` when all gates pass, or an `Err` describing the first
/// violation found.
pub fn check_implementation_gates(outcome: &OutcomeSpec, manifest: &TestManifest) -> Result<()> {
    // Gate 1 — outcome must be actionable
    if outcome.status != OutcomeStatus::Pending && outcome.status != OutcomeStatus::Active {
        anyhow::bail!(
            "outcome '{}' has status {:?} and cannot be implemented",
            outcome.id,
            outcome.status
        );
    }

    // Collect tests for this outcome
    let outcome_tests: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| t.feature_id == outcome.feature_id && t.outcome_id == outcome.id)
        .collect();

    // Gate 2 — at least one test must exist
    if outcome_tests.is_empty() {
        anyhow::bail!(
            "outcome '{}' has no tests in the manifest — add tests with \
             `specrail test add` before implementing",
            outcome.id
        );
    }

    // Gate 3 — no test may be in `planned` status
    let planned: Vec<_> = outcome_tests
        .iter()
        .filter(|t| t.status == TestStatus::Planned)
        .map(|t| t.id.as_str())
        .collect();

    if !planned.is_empty() {
        anyhow::bail!(
            "outcome '{}' has tests still in 'planned' status: [{}]\n\
             Update test status to 'written' once the test file exists.",
            outcome.id,
            planned.join(", ")
        );
    }

    Ok(())
}

/// Check whether all policy gates pass for advancing past the current outcome.
///
/// Rules enforced:
///
/// 1. The outcome must be in `verified` status.
pub fn check_advance_gates(outcome: &OutcomeSpec) -> Result<()> {
    if outcome.status != OutcomeStatus::Verified {
        anyhow::bail!(
            "outcome '{}' is not verified (status: {:?}) — run `specrail verify` first",
            outcome.id,
            outcome.status
        );
    }
    Ok(())
}
