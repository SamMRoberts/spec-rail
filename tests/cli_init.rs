use assert_cmd::Command;
use predicates::prelude::*;
use serde_yaml::Value;
use std::fs;
use tempfile::TempDir;

mod support;

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
    assert!(base.join("specrail.db").exists(), "specrail.db missing");
    assert!(base.join("state").is_dir(), "state/ missing");
    assert!(base.join("agents").is_dir(), "agents/ missing");
    assert!(!base.join("features").exists(), "features/ should not exist");
    assert!(!base.join("outcomes").exists(), "outcomes/ should not exist");
    assert!(!base.join("tests").exists(), "tests/ should not exist");
    assert!(!base.join("state/current.yaml").exists(), "current.yaml should not exist");
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
fn init_uses_rust_test_command_when_cargo_toml_exists() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname='demo'\nversion='0.1.0'\n").unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    let content = fs::read_to_string(dir.path().join(".specrail/project.yaml")).unwrap();
    let yaml: Value = serde_yaml::from_str(&content).unwrap();
    assert_eq!(yaml["test_command"].as_str(), Some("cargo test"));
}

#[test]
fn init_uses_dotnet_test_command_when_csproj_exists() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("src/App")).unwrap();
    fs::create_dir_all(dir.path().join("tests/App.Tests")).unwrap();
    fs::write(
        dir.path().join("src/App/App.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>",
    )
    .unwrap();
    fs::write(
        dir.path().join("tests/App.Tests/App.Tests.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>",
    )
    .unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    let content = fs::read_to_string(dir.path().join(".specrail/project.yaml")).unwrap();
    let yaml: Value = serde_yaml::from_str(&content).unwrap();
    assert_eq!(
        yaml["test_command"].as_str(),
        Some("dotnet test --project tests/App.Tests/App.Tests.csproj")
    );
}

#[test]
fn init_creates_database_artifact() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    assert!(dir.path().join(".specrail/specrail.db").exists());
}

#[test]
fn init_ledger_gets_project_initialized_event() {
    let dir = TempDir::new().unwrap();
    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    let history = support::history(&dir);
    assert!(history.iter().any(|event| event.event_type == "project_initialized"));
}

#[test]
fn help_lists_mcp_server_command() {
    let dir = TempDir::new().unwrap();

    specrail(&dir)
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("mcp-server"));
}

#[test]
fn mcpserver_alias_resolves_to_mcp_server_command() {
    let dir = TempDir::new().unwrap();

    specrail(&dir)
        .args(["mcpserver", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run the specrail MCP server over stdio"));
}
