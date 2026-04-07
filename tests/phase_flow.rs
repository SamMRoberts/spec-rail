use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

fn setup(dir: &TempDir) {
    let bin = "specrail";
    Command::cargo_bin(bin)
        .unwrap()
        .current_dir(dir.path())
        .arg("init")
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
fn phase_new_creates_file() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "phase", "new", "auth-login", "phase-1-domain",
            "--title", "Domain Validation",
            "--goal", "Establish domain invariants.",
            "--order", "1",
        ])
        .assert()
        .success();

    let path = dir
        .path()
        .join(".specrail/phases/auth-login/phase-1-domain.yaml");
    assert!(path.exists(), "phase file not created");
}

#[test]
fn phase_new_records_ledger_event() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "phase", "new", "auth-login", "phase-1-domain",
            "--title", "Domain Validation",
            "--goal", "Establish domain invariants.",
            "--order", "1",
        ])
        .assert()
        .success();

    let ledger =
        fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(ledger.contains("phase_created"));
    assert!(ledger.contains("phase-1-domain"));
}

#[test]
fn phase_list_shows_phases() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "phase", "new", "auth-login", "phase-1-domain",
            "--title", "Domain", "--goal", "g", "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["phase", "list", "auth-login"])
        .assert()
        .success()
        .stdout(contains("phase-1-domain"));
}

#[test]
fn phase_activate_updates_state() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "phase", "new", "auth-login", "phase-1-domain",
            "--title", "Domain", "--goal", "g", "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["phase", "activate", "auth-login", "phase-1-domain"])
        .assert()
        .success();

    let state =
        fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("phase-1-domain"), "state should reference active phase");
    assert!(state.contains("auth-login"), "state should reference active feature");
}

#[test]
fn phase_new_fails_for_unknown_feature() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "phase", "new", "nonexistent-feature", "phase-1",
            "--title", "T", "--goal", "g", "--order", "1",
        ])
        .assert()
        .failure();
}

#[test]
fn phase_show_displays_details() {
    let dir = TempDir::new().unwrap();
    setup(&dir);

    specrail(&dir)
        .args([
            "phase", "new", "auth-login", "phase-1-domain",
            "--title", "Domain Validation",
            "--goal", "Establish auth domain invariants.",
            "--order", "1",
            "--allow", "src/domain/**",
            "--forbid", "src/http/**",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["phase", "show", "auth-login", "phase-1-domain"])
        .assert()
        .success()
        .stdout(contains("Domain Validation"))
        .stdout(contains("Establish auth domain invariants."))
        .stdout(contains("src/domain/**"));
}
