use assert_cmd::Command;
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

    let feature = fs::read_to_string(dir.path().join(".specrail/features/auth.yaml")).unwrap();
    assert!(feature.contains("Authentication"));
    assert!(feature.contains("Users can sign in"));
    assert!(feature.contains("outcome-2"), "feature should point to the active outcome");

    let outcome_one = fs::read_to_string(dir.path().join(".specrail/outcomes/auth/outcome-1.yaml"))
        .unwrap();
    assert!(outcome_one.contains("src/auth/**"));
    assert!(outcome_one.contains("tests/auth/validate.rs"));

    let outcome_two = fs::read_to_string(dir.path().join(".specrail/outcomes/auth/outcome-2.yaml"))
        .unwrap();
    assert!(outcome_two.contains("outcome-1"));
    assert!(outcome_two.contains("src/auth/persistence/**"));

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

    assert!(
        dir.path().join(".specrail/features/auth.yaml").exists(),
        "first feature should exist"
    );
    assert!(
        dir.path().join(".specrail/features/billing.yaml").exists(),
        "second feature should exist"
    );

    let state = fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("billing"), "last feature should be active");
    assert!(state.contains("outcome-1"), "last feature outcome should be active");
}