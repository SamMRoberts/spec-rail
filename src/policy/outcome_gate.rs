use std::path::PathBuf;

use anyhow::Result;

use crate::core::{
    models::{OutcomeSpec, OutcomeStatus, TestManifest, TestStatus},
    repository::Repository,
};

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
pub fn check_implementation_gates(
    repo: &Repository,
    outcome: &OutcomeSpec,
    manifest: &TestManifest,
) -> Result<()> {
    // Gate 1 — outcome must be actionable
    if outcome.status != OutcomeStatus::Pending && outcome.status != OutcomeStatus::Active {
        anyhow::bail!(
            "outcome '{}' has status {:?} and cannot be implemented",
            outcome.id,
            outcome.status
        );
    }

    check_test_readiness(repo, outcome, manifest)
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

/// Check whether verification can run against the active outcome.
///
/// Verification uses the project's full test command, so it still relies on the
/// same per-outcome readiness checks as implementation to avoid recording noisy
/// or misleading results when required tests are missing, still planned, or not
/// yet written to disk.
pub fn check_verify_gates(
    repo: &Repository,
    outcome: &OutcomeSpec,
    manifest: &TestManifest,
) -> Result<()> {
    check_test_readiness(repo, outcome, manifest)
}

/// Enforce the shared test-readiness rules that must hold before workflow steps
/// can trust the current outcome's tests as a meaningful gate.
///
/// This requires:
/// - at least one registered outcome test,
/// - declared required test IDs,
/// - discoverable required test files,
/// - manifest coverage for each required test ID and file,
/// - no outcome tests left in `planned`,
/// - and required test files that already exist on disk.
fn check_test_readiness(repo: &Repository, outcome: &OutcomeSpec, manifest: &TestManifest) -> Result<()> {
    let outcome_tests: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| t.feature_id == outcome.feature_id && t.outcome_id == outcome.id)
        .collect();

    if outcome_tests.is_empty() {
        anyhow::bail!(
            "outcome '{}' has no tests in the manifest — add tests with \
             `specrail test add` before continuing",
            outcome.id
        );
    }

    if outcome.required_tests.is_empty() {
        anyhow::bail!(
            "outcome '{}' has no required test IDs — review the outcome and register the tests before continuing",
            outcome.id
        );
    }

    if outcome.required_test_files.is_empty() {
        anyhow::bail!(
            "outcome '{}' has no required test files — review the outcome and register test paths before continuing",
            outcome.id
        );
    }

    let missing_required_tests: Vec<_> = outcome
        .required_tests
        .iter()
        .filter(|required_test_id| !outcome_tests.iter().any(|test| test.id == **required_test_id))
        .cloned()
        .collect();
    if !missing_required_tests.is_empty() {
        anyhow::bail!(
            "outcome '{}' is missing registered required tests: [{}]\nRun `specrail test add` or `specrail_outcome_test_review` before continuing.",
            outcome.id,
            missing_required_tests.join(", ")
        );
    }

    let missing_required_test_files: Vec<_> = outcome
        .required_test_files
        .iter()
        .filter(|required_path| !outcome_tests.iter().any(|test| test.path == **required_path))
        .cloned()
        .collect();
    if !missing_required_test_files.is_empty() {
        anyhow::bail!(
            "outcome '{}' is missing registered required test files: [{}]\nRun `specrail test add` or `specrail_outcome_test_review` before continuing.",
            outcome.id,
            missing_required_test_files.join(", ")
        );
    }

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

    let missing_files: Vec<PathBuf> = outcome
        .required_test_files
        .iter()
        .map(|path| repo.root.join(path))
        .filter(|path| !path.is_file())
        .collect();

    if !missing_files.is_empty() {
        let display_paths: Vec<_> = missing_files
            .iter()
            .map(|path| path.strip_prefix(&repo.root).unwrap_or(path).display().to_string())
            .collect();
        anyhow::bail!(
            "outcome '{}' has required test files that do not exist yet: [{}]\nCreate the test files before continuing.",
            outcome.id,
            display_paths.join(", ")
        );
    }

    Ok(())
}
