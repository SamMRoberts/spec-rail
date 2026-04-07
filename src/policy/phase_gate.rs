use anyhow::Result;

use crate::core::models::{PhaseSpec, PhaseStatus, TestManifest, TestStatus};

/// Check whether all policy gates pass for entering the implementation step of
/// the given phase.
///
/// Rules enforced:
///
/// 1. The phase must be in `pending` or `active` status.
/// 2. At least one test must be registered for the phase in the manifest.
/// 3. All registered tests must be in `written`, `passing`, or `failing`
///    status — **not** `planned`.  Tests must exist before implementation.
///
/// Returns `Ok(())` when all gates pass, or an `Err` describing the first
/// violation found.
pub fn check_implementation_gates(phase: &PhaseSpec, manifest: &TestManifest) -> Result<()> {
    // Gate 1 — phase must be actionable
    if phase.status != PhaseStatus::Pending && phase.status != PhaseStatus::Active {
        anyhow::bail!(
            "phase '{}' has status {:?} and cannot be implemented",
            phase.id,
            phase.status
        );
    }

    // Collect tests for this phase
    let phase_tests: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| t.phase_id == phase.id)
        .collect();

    // Gate 2 — at least one test must exist
    if phase_tests.is_empty() {
        anyhow::bail!(
            "phase '{}' has no tests in the manifest — add tests with \
             `specrail test add` before implementing",
            phase.id
        );
    }

    // Gate 3 — no test may be in `planned` status
    let planned: Vec<_> = phase_tests
        .iter()
        .filter(|t| t.status == TestStatus::Planned)
        .map(|t| t.id.as_str())
        .collect();

    if !planned.is_empty() {
        anyhow::bail!(
            "phase '{}' has tests still in 'planned' status: [{}]\n\
             Update test status to 'written' once the test file exists.",
            phase.id,
            planned.join(", ")
        );
    }

    Ok(())
}

/// Check whether all policy gates pass for advancing past the current phase.
///
/// Rules enforced:
///
/// 1. The phase must be in `verified` status.
pub fn check_advance_gates(phase: &PhaseSpec) -> Result<()> {
    if phase.status != PhaseStatus::Verified {
        anyhow::bail!(
            "phase '{}' is not verified (status: {:?}) — run `specrail verify` first",
            phase.id,
            phase.status
        );
    }
    Ok(())
}
