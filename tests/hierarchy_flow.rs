use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

mod support;

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

    let state = support::current_state(&dir);
    assert_eq!(state.active_solution.as_deref(), Some("default-solution"));
    assert_eq!(state.active_project.as_deref(), Some("default-project"));
    assert_eq!(state.active_component.as_deref(), Some("default-component"));
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

    let state = support::current_state(&dir);
    assert_eq!(state.active_solution.as_deref(), Some("platform"));
    assert_eq!(state.active_project.as_deref(), Some("api"));
    assert_eq!(state.active_component.as_deref(), Some("billing"));
    assert_eq!(state.active_feature.as_deref(), Some("invoice-sync"));
}
