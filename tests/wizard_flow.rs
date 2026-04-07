use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn specrail(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("specrail").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn init_walkthrough_creates_feature_and_multiple_phases() {
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
             phase-1\n\
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
             phase-2\n\
             Persistence\n\
             Persist authenticated users.\n\
             \n\
             phase-1\n\
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
    assert!(feature.contains("phase-2"), "feature should point to the active phase");

    let phase_one = fs::read_to_string(dir.path().join(".specrail/phases/auth/phase-1.yaml"))
        .unwrap();
    assert!(phase_one.contains("src/auth/**"));
    assert!(phase_one.contains("tests/auth/validate.rs"));

    let phase_two = fs::read_to_string(dir.path().join(".specrail/phases/auth/phase-2.yaml"))
        .unwrap();
    assert!(phase_two.contains("phase-1"));
    assert!(phase_two.contains("src/auth/persistence/**"));

    let state = fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("auth"));
    assert!(state.contains("phase-2"));

    let ledger = fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(ledger.contains("feature_created"));
    assert!(ledger.contains("phase_created"));
    assert!(ledger.contains("feature_activated"));
    assert!(ledger.contains("phase_activated"));
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
             phase-1\n\
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
             phase-1\n\
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
    assert!(state.contains("phase-1"), "last feature phase should be active");
}