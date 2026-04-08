use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

fn init(dir: &TempDir) {
    Command::cargo_bin("specrail")
        .unwrap()
        .current_dir(dir.path())
        .args(["init", "--no-wizard"])
        .assert()
        .success();
}

fn specrail(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("specrail").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn init_bootstraps_default_solution_project_and_component() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args(["solution", "show", "default-solution"])
        .assert()
        .success()
        .stdout(contains("Default Solution"));
    specrail(&dir)
        .args(["project", "show", "default-project"])
        .assert()
        .success()
        .stdout(contains("Default Project"));
    specrail(&dir)
        .args(["component", "show", "default-component"])
        .assert()
        .success()
        .stdout(contains("Default Component"));

    let state = fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("active_solution: default-solution"));
    assert!(state.contains("active_project: default-project"));
    assert!(state.contains("active_component: default-component"));
}

#[test]
fn feature_can_be_scoped_to_an_explicit_component_hierarchy() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "solution",
            "new",
            "platform",
            "--title",
            "Platform",
            "--purpose",
            "Overall product solution.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "project",
            "new",
            "platform",
            "api",
            "--title",
            "API",
            "--purpose",
            "Primary API project.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "component",
            "new",
            "api",
            "billing",
            "--title",
            "Billing",
            "--purpose",
            "Billing component.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "feature",
            "new",
            "invoice-sync",
            "--component",
            "billing",
            "--title",
            "Invoice Sync",
            "--purpose",
            "Synchronize invoices.",
        ])
        .assert()
        .success()
        .stdout(contains("platform/api/billing"));

    specrail(&dir)
        .args(["feature", "show", "invoice-sync"])
        .assert()
        .success()
        .stdout(contains("Hierarchy: platform/api/billing"));

    specrail(&dir)
        .args(["feature", "activate", "invoice-sync"])
        .assert()
        .success();

    let state = fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("active_solution: platform"));
    assert!(state.contains("active_project: api"));
    assert!(state.contains("active_component: billing"));
    assert!(state.contains("active_feature: invoice-sync"));
}
