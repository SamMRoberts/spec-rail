use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

fn specrail(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("specrail").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn init_walkthrough_creates_feature_and_multiple_outcomes() {
    let dir = TempDir::new().unwrap();

    specrail(&dir)
        .arg("init")
        .write_stdin(
            "auth\n\
             Authentication\n\
             Authenticate users before protected actions.\n\
             Users can sign in\n\
             \n\
             Keep auth logic isolated\n\
             \n\
             Do not build billing\n\
             \n\
             sessions\n\
             \n\
             outcome-1\n\
             Validation\n\
             Validate credentials and reject bad input.\n\
             \n\
             \n\
             src/auth/**\n\
             \n\
             src/billing/**\n\
             \n\
             tests/auth/validate.rs\n\
             \n\
             y\n\
             outcome-2\n\
             Persistence\n\
             Persist authenticated users.\n\
             \n\
             outcome-1\n\
             \n\
             src/auth/persistence/**\n\
             \n\
             src/http/**\n\
             \n\
             tests/auth/persist.rs\n\
             \n\
             n\n\
             n\n\
             n\n",
        )
        .assert()
        .success();

    specrail(&dir)
        .args(["feature", "show", "auth"])
        .assert()
        .success()
        .stdout(contains("Authentication"))
        .stdout(contains("Users can sign in"))
        .stdout(contains("Current outcome: outcome-2"));

    specrail(&dir)
        .args(["outcome", "show", "auth", "outcome-1"])
        .assert()
        .success()
        .stdout(contains("src/auth/**"))
        .stdout(contains("tests/auth/validate.rs"));

    specrail(&dir)
        .args(["outcome", "show", "auth", "outcome-2"])
        .assert()
        .success()
        .stdout(contains("outcome-1"))
        .stdout(contains("src/auth/persistence/**"));

    let state = fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("auth"));
    assert!(state.contains("outcome-2"));

    let ledger = fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(ledger.contains("feature_created"));
    assert!(ledger.contains("outcome_created"));
    assert!(ledger.contains("feature_activated"));
    assert!(ledger.contains("outcome_activated"));
}

#[test]
fn init_walkthrough_can_repeat_features() {
    let dir = TempDir::new().unwrap();

    specrail(&dir)
        .arg("init")
        .write_stdin(
            "auth\n\
             Authentication\n\
             Authenticate users.\n\
             \n\
             \n\
             \n\
             \n\
             outcome-1\n\
             Validation\n\
             Validate input.\n\
             \n\
             \n\
             \n\
             \n\
             \n\
             n\n\
             y\n\
             billing\n\
             Billing\n\
             Charge customers.\n\
             \n\
             \n\
             \n\
             \n\
             outcome-1\n\
             Capture\n\
             Capture payment details.\n\
             \n\
             \n\
             \n\
             \n\
             \n\
             n\n\
             n\n\
             n\n",
        )
        .assert()
        .success();

    specrail(&dir)
        .args(["feature", "list"])
        .assert()
        .success()
        .stdout(contains("auth"))
        .stdout(contains("billing"));

    let state = fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("billing"), "last feature should be active");
    assert!(state.contains("outcome-1"), "last feature outcome should be active");
}