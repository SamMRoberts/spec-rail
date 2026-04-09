mod support;

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

fn specrail(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("specrail").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

/// Set up a project with one feature, two outcomes, and one test in the manifest.
fn full_setup(dir: &TempDir) {
    specrail(dir).args(["init", "--no-wizard"]).assert().success();

    specrail(dir)
        .args([
            "feature", "new", "auth",
            "--title", "Auth", "--purpose", "Authenticate users.",
        ])
        .assert()
        .success();

    specrail(dir)
        .args([
            "outcome", "new", "auth", "outcome-1",
            "--title", "Outcome 1", "--goal", "Validate inputs.", "--order", "1",
        ])
        .assert()
        .success();

    specrail(dir)
        .args([
            "outcome", "new", "auth", "outcome-2",
            "--title", "Outcome 2", "--goal", "Persist user.", "--order", "2",
        ])
        .assert()
        .success();

    specrail(dir)
        .args([
            "test", "add", "test-validate-email",
            "--feature", "auth",
            "--outcome", "outcome-1",
            "--path", "tests/domain/validate_email.rs",
            "--kind", "unit",
        ])
        .assert()
        .success();

    // Mark test as written so the outcome gate allows implementation
    specrail(dir)
        .args(["test", "set-status", "test-validate-email", "written"])
        .assert()
        .success();

    specrail(dir)
        .args(["feature", "activate", "auth"])
        .assert()
        .success();

    specrail(dir)
        .args(["outcome", "activate", "auth", "outcome-1"])
        .assert()
        .success();
}

// ── implement gate tests ───────────────────────────────────────────────────────

#[test]
fn implement_fails_when_no_active_feature() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir).arg("implement").assert().failure();
}

#[test]
fn implement_fails_when_no_tests_in_manifest() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir)
        .args([
            "feature", "new", "feat",
            "--title", "F", "--purpose", "p",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "new", "feat", "o1",
            "--title", "T", "--goal", "g", "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["feature", "activate", "feat"])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "activate", "feat", "o1"])
        .assert()
        .success();

    // Should fail: no tests registered
    specrail(&dir).arg("implement").assert().failure();
}

#[test]
fn implement_fails_when_tests_still_planned() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir)
        .args([
            "feature", "new", "feat",
            "--title", "F", "--purpose", "p",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "new", "feat", "o1",
            "--title", "T", "--goal", "g", "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "test", "add", "my-test",
            "--feature", "feat", "--outcome", "o1",
            "--path", "tests/my_test.rs",
        ])
        .assert()
        .success();
    // Status remains "planned"

    specrail(&dir)
        .args(["feature", "activate", "feat"])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "activate", "feat", "o1"])
        .assert()
        .success();

    specrail(&dir).arg("implement").assert().failure();
}

// ── verify ────────────────────────────────────────────────────────────────────

#[test]
fn verify_fails_when_no_active_outcome() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();
    specrail(&dir).arg("verify").assert().failure();
}

// ── advance ───────────────────────────────────────────────────────────────────

#[test]
fn advance_fails_when_outcome_not_verified() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    // Outcome is active but not verified
    specrail(&dir).arg("advance").assert().failure();
}

// ── status ────────────────────────────────────────────────────────────────────

#[test]
fn status_shows_project_info() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    specrail(&dir)
        .arg("status")
        .assert()
        .success()
        .stdout(contains("auth"))
        .stdout(contains("outcome-1"));
}

// ── trace ─────────────────────────────────────────────────────────────────────

#[test]
fn trace_shows_ledger_events() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    specrail(&dir)
        .arg("trace")
        .assert()
        .success()
        .stdout(contains("project_initialized"))
        .stdout(contains("feature_created"))
        .stdout(contains("outcome_created"))
        .stdout(contains("test_added"));
}

#[test]
fn trace_limit_restricts_output() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    let output = specrail(&dir)
        .args(["trace", "--limit", "1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Only the header line + 1 data line = 2 non-empty lines (plus separator)
    let lines: Vec<&str> = std::str::from_utf8(&output)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('─'))
        .collect();
    // Header + 1 event
    assert_eq!(lines.len(), 2, "expected header + 1 event, got {lines:?}");
}

// ── test manifest ─────────────────────────────────────────────────────────────

#[test]
fn test_list_filters_by_feature() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    specrail(&dir)
        .args(["test", "list", "--feature", "auth"])
        .assert()
        .success()
        .stdout(contains("test-validate-email"));
}

#[test]
fn test_set_status_updates_manifest() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    specrail(&dir)
        .args(["test", "set-status", "test-validate-email", "passing"])
        .assert()
        .success();

    let tests = support::tests(&dir);
    assert!(tests.iter().any(|test| {
        test.id == "test-validate-email" && test.status == "passing"
    }));
}

// ── verify + advance happy path ───────────────────────────────────────────────

/// This test patches the per-project settings file so the test_command is
/// `true` (always exits 0), allowing us to exercise the verify → advance path
/// without a real test suite.
#[test]
fn verify_and_advance_happy_path() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    // Write per-project settings for default-project with test_command: true
    let project_config_dir = dir.path().join(".specrail/projects");
    fs::create_dir_all(&project_config_dir).unwrap();
    fs::write(
        project_config_dir.join("default-project.yaml"),
        "test_command: 'true'\n",
    )
    .unwrap();

    // Verify should now pass
    specrail(&dir).arg("verify").assert().success();

    // Advance should succeed
    specrail(&dir).arg("advance").assert().success();

    // State should now reference outcome-2
    let state = support::current_state(&dir);
    assert_eq!(state.active_outcome.as_deref(), Some("outcome-2"));

    // Ledger should show advancement
    let history = support::history(&dir);
    assert!(history.iter().any(|event| event.event_type == "outcome_advanced"));
    assert!(history.iter().any(|event| event.event_type == "outcome_verified"));
}

/// Test that verify marks outcome as failed when test_command exits non-zero.
#[test]
fn verify_marks_outcome_failed_on_test_failure() {
    let dir = TempDir::new().unwrap();
    full_setup(&dir);

    // Patch per-project settings for default-project with a command that always fails
    let project_config_dir = dir.path().join(".specrail/projects");
    fs::create_dir_all(&project_config_dir).unwrap();
    fs::write(
        project_config_dir.join("default-project.yaml"),
        "test_command: 'false'\n",
    )
    .unwrap();

    // Verify exits with success (CLI completes) but records failure
    specrail(&dir).arg("verify").assert().success();

    specrail(&dir)
        .args(["outcome", "show", "auth", "outcome-1"])
        .assert()
        .success()
        .stdout(contains("Status:  Failed"));
}

