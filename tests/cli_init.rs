use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn specrail(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("specrail").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn init_creates_directory_structure() {
    let dir = TempDir::new().unwrap();

    specrail(&dir)
        .args(["init", "--no-wizard"])
        .assert()
        .success();

    let base = dir.path().join(".specrail");
    assert!(base.is_dir(), ".specrail/ not created");
    assert!(base.join("project.yaml").exists(), "project.yaml missing");
    assert!(base.join("features").is_dir(), "features/ missing");
    assert!(base.join("phases").is_dir(), "phases/ missing");
    assert!(base.join("tests").is_dir(), "tests/ missing");
    assert!(base.join("tests/manifest.yaml").exists(), "manifest.yaml missing");
    assert!(base.join("state").is_dir(), "state/ missing");
    assert!(base.join("state/current.yaml").exists(), "current.yaml missing");
    assert!(base.join("state/ledger.jsonl").exists(), "ledger.jsonl missing");
    assert!(base.join("agents").is_dir(), "agents/ missing");
}

#[test]
fn init_is_idempotent() {
    let dir = TempDir::new().unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    // Write a canary file to verify it is not overwritten
    let canary = dir.path().join(".specrail/project.yaml");
    let original = fs::read_to_string(&canary).unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    let after = fs::read_to_string(&canary).unwrap();
    assert_eq!(original, after, "project.yaml was overwritten on second init");
}

#[test]
fn init_creates_valid_project_yaml() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    let content = fs::read_to_string(dir.path().join(".specrail/project.yaml")).unwrap();
    assert!(content.contains("version:"), "version field missing");
    assert!(content.contains("test_command:"), "test_command field missing");
    assert!(content.contains("default_agent:"), "default_agent field missing");
}

#[test]
fn init_creates_valid_manifest_yaml() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    let content =
        fs::read_to_string(dir.path().join(".specrail/tests/manifest.yaml")).unwrap();
    assert!(content.contains("tests:"), "manifest.yaml should contain 'tests:' key");
}

#[test]
fn init_ledger_gets_project_initialized_event() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    let ledger =
        fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(
        ledger.contains("project_initialized"),
        "ledger should contain project_initialized event"
    );
}
