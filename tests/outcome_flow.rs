mod support;

use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

fn setup(dir: &TempDir) {
    let bin = "specrail";
    Command::cargo_bin(bin)
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--no-wizard"])
        .assert()
        .success();

    Command::cargo_bin(bin)
        .unwrap()
        .current_dir(dir.path())
        .args([
            "feature", "new", "auth-login",
            "--title", "User Login",
            "--purpose", "Allow a user to authenticate.",
        ])
        .assert()
        .success();
}

fn specrail(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("specrail").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn outcome_new_persists_record() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "Establish domain invariants.",
            "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("Domain Validation"))
        .stdout(contains("Establish domain invariants."));
}

#[test]
fn outcome_new_records_ledger_event() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "Establish domain invariants.",
            "--order", "1",
        ])
        .assert()
        .success();

    let ledger =
        fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(ledger.contains("outcome_created"));
    assert!(ledger.contains("outcome-1-domain"));
}

#[test]
fn outcome_list_shows_outcomes() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain", "--goal", "g", "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "list", "auth-login"])
        .assert()
        .success()
        .stdout(contains("outcome-1-domain"));
}

#[test]
fn outcome_activate_updates_state() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain", "--goal", "g", "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "activate", "auth-login", "outcome-1-domain"])
        .assert()
        .success();

    let state = support::current_state(&dir);
    assert_eq!(state.active_outcome.as_deref(), Some("outcome-1-domain"));
    assert_eq!(state.active_feature.as_deref(), Some("auth-login"));
}

#[test]
fn outcome_new_prints_field_explanations_and_next_steps() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "User can log in with valid credentials.",
            "--order", "1",
        ])
        .assert()
        .success()
        .stdout(contains("What each field does:"))
        .stdout(contains("goal   — acceptance criterion"))
        .stdout(contains("order  — sequence position"))
        .stdout(contains("allow  — glob paths the AI agent may modify"))
        .stdout(contains("test   — required test names/facts that must pass"))
        .stdout(contains("test-file — test file paths used by `specrail test generate`"))
        .stdout(contains("Next steps:"))
        .stdout(contains("specrail test add"))
        .stdout(contains("specrail outcome activate auth-login outcome-1-domain"));
}

#[test]
fn outcome_show_prints_contextual_labels() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "User can log in with valid credentials.",
            "--order", "1",
            "--allow", "src/auth/**",
            "--forbid", "src/billing/**",
            "--test", "validates_credentials",
            "--test-file", "tests/auth/validate.rs",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("order controls the sequence used by `specrail advance`"))
        .stdout(contains("goal is the acceptance criterion"))
        .stdout(contains("AI agent may only modify"))
        .stdout(contains("AI agent must NOT touch"))
        .stdout(contains("Required test names / facts:"))
        .stdout(contains("No required test file paths set."))
        .stdout(contains("add individual tests with `specrail test add`"));
}

#[test]
fn outcome_show_suggests_test_add_when_no_required_tests() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "User can log in.",
            "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("No required test names set."))
        .stdout(contains("No required test file paths set."))
        .stdout(contains("specrail test add"));
}

#[test]
fn outcome_new_fails_for_unknown_feature() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "nonexistent-feature", "outcome-1",
            "--title", "T", "--goal", "g", "--order", "1",
        ])
        .assert()
        .failure();
}

#[test]
fn outcome_show_displays_details() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "Establish auth domain invariants.",
            "--order", "1",
            "--allow", "src/domain/**",
            "--forbid", "src/http/**",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("Domain Validation"))
        .stdout(contains("Establish auth domain invariants."))
        .stdout(contains("src/domain/**"));
}

#[test]
fn outcome_edit_updates_existing_record() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "Initial goal.",
            "--order", "1",
            "--allow", "src/domain/**",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "edit", "auth-login", "outcome-1-domain",
            "--title", "Credential Validation",
            "--goal", "Updated goal.",
            "--order", "2",
            "--allow", "src/auth/**",
            "--forbid", "src/http/**",
            "--test", "validates_credentials",
            "--test-file", "tests/auth_login.rs",
        ])
        .assert()
        .success()
        .stdout(contains("updated"));

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("Credential Validation"))
        .stdout(contains("Updated goal."))
        .stdout(contains("Outcome: outcome-1-domain (order 2)"))
        .stdout(contains("src/auth/**"))
        .stdout(contains("No required test file paths set."));

    let ledger = fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(ledger.contains("outcome_edited"));
}

#[test]
fn outcome_edit_resets_verified_status_to_pending() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "Initial goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    support::set_outcome_status(&dir, "auth-login", "outcome-1-domain", "verified");

    specrail(&dir)
        .args([
            "outcome", "edit", "auth-login", "outcome-1-domain",
            "--title", "Credential Validation",
            "--goal", "Updated goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("Status:  Pending"));
}

#[test]
fn outcome_edit_resets_legacy_complete_status_to_pending() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "Initial goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    support::set_outcome_status(&dir, "auth-login", "outcome-1-domain", "complete");

    specrail(&dir)
        .args([
            "outcome", "edit", "auth-login", "outcome-1-domain",
            "--title", "Credential Validation",
            "--goal", "Updated goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("Status:  Pending"));
}

#[test]
fn outcome_edit_resets_legacy_completed_status_to_pending() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "outcome", "new", "auth-login", "outcome-1-domain",
            "--title", "Domain Validation",
            "--goal", "Initial goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    support::set_outcome_status(&dir, "auth-login", "outcome-1-domain", "completed");

    specrail(&dir)
        .args([
            "outcome", "edit", "auth-login", "outcome-1-domain",
            "--title", "Credential Validation",
            "--goal", "Updated goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["outcome", "show", "auth-login", "outcome-1-domain"])
        .assert()
        .success()
        .stdout(contains("Status:  Pending"));
}
