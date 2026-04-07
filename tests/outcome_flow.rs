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
fn outcome_new_creates_file() {
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

    let path = dir
        .path()
        .join(".specrail/outcomes/auth-login/outcome-1-domain.yaml");
    assert!(path.exists(), "outcome file not created");
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

    let state =
        fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("outcome-1-domain"), "state should reference active outcome");
    assert!(state.contains("auth-login"), "state should reference active feature");
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
fn outcome_edit_updates_existing_file() {
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
            "--test", "tests/auth_login.rs",
        ])
        .assert()
        .success()
        .stdout(contains("updated"));

    let outcome = fs::read_to_string(
        dir.path().join(".specrail/outcomes/auth-login/outcome-1-domain.yaml"),
    )
    .unwrap();
    assert!(outcome.contains("Credential Validation"));
    assert!(outcome.contains("Updated goal."));
    assert!(outcome.contains("order: 2"));
    assert!(outcome.contains("src/auth/**"));
    assert!(outcome.contains("tests/auth_login.rs"));

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

    let path = dir
        .path()
        .join(".specrail/outcomes/auth-login/outcome-1-domain.yaml");
    let updated = fs::read_to_string(&path)
        .unwrap()
        .replace("status: pending", "status: verified");
    fs::write(&path, updated).unwrap();

    specrail(&dir)
        .args([
            "outcome", "edit", "auth-login", "outcome-1-domain",
            "--title", "Credential Validation",
            "--goal", "Updated goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    let outcome = fs::read_to_string(&path).unwrap();
    assert!(outcome.contains("status: pending"));
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

    let path = dir
        .path()
        .join(".specrail/outcomes/auth-login/outcome-1-domain.yaml");
    let updated = fs::read_to_string(&path)
        .unwrap()
        .replace("status: pending", "status: complete");
    fs::write(&path, updated).unwrap();

    specrail(&dir)
        .args([
            "outcome", "edit", "auth-login", "outcome-1-domain",
            "--title", "Credential Validation",
            "--goal", "Updated goal.",
            "--order", "1",
        ])
        .assert()
        .success();

    let outcome = fs::read_to_string(&path).unwrap();
    assert!(outcome.contains("status: pending"));
}
