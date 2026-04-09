mod support;

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
fn test_generate_creates_files_and_manifest_entries() {
    let dir = TempDir::new().unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir)
        .args([
            "feature", "new", "auth",
            "--title", "Auth",
            "--purpose", "Authenticate users.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "new", "auth", "outcome-1",
            "--title", "Validation",
            "--goal", "Validate credentials.",
            "--order", "1",
            "--test", "auth-outcome-1-validates-credentials",
            "--test-file", "tests/auth/validate.rs",
        ])
        .assert()
        .success();

    fs::create_dir_all(dir.path().join(".specrail/agents")).unwrap();
    fs::write(
        dir.path().join(".specrail/agents/mock_test_output.json"),
                r##"{
  "tests": [
    {
      "feature_id": "auth",
      "outcome_id": "outcome-1",
            "id": "auth-outcome-1-validates-credentials",
            "name": "validates_credentials",
      "path": "tests/auth/validate.rs",
      "kind": "unit",
      "purpose_refs": ["goal:Validate credentials."],
      "content": "#[test]\nfn validates_credentials() {\n    assert!(true);\n}\n"
    }
  ]
}"##,
    )
    .unwrap();

    specrail(&dir)
        .env("SPECRAIL_AGENT_CMD", "cat .specrail/agents/mock_test_output.json")
        .args(["test", "generate", "--agent", "generic-shell"])
        .assert()
        .success()
        .stdout(contains("Generated 1 test file(s)"));

    let test_file = fs::read_to_string(dir.path().join("tests/auth/validate.rs")).unwrap();
    assert!(test_file.contains("validates_credentials"));

    let tests = support::tests(&dir);
    assert!(tests.iter().any(|test| {
        test.id == "auth-outcome-1-validates-credentials"
            && test.name == "validates_credentials"
            && test.path == "tests/auth/validate.rs"
            && test.status == "written"
    }));
    assert_eq!(
        support::outcome_required_tests(&dir, "auth", "outcome-1"),
        vec!["auth-outcome-1-validates-credentials".to_string()]
    );

    let history = support::history(&dir);
    assert!(history.iter().any(|event| event.event_type == "test_generation_run"));
    assert!(history.iter().any(|event| {
        event.message
            .as_deref()
            .is_some_and(|message| message.contains("generated"))
    }));
}

#[test]
fn test_generate_promotes_existing_planned_test_to_written() {
    let dir = TempDir::new().unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir)
        .args([
            "feature", "new", "auth",
            "--title", "Auth",
            "--purpose", "Authenticate users.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "new", "auth", "outcome-1",
            "--title", "Validation",
            "--goal", "Validate credentials.",
            "--order", "1",
            "--test", "auth-outcome-1-validates-credentials",
            "--test-file", "tests/auth/validate.rs",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "test", "add", "auth-outcome-1-validates-credentials",
            "--feature", "auth",
            "--outcome", "outcome-1",
            "--path", "tests/auth/validate.rs",
            "--kind", "unit",
        ])
        .assert()
        .success();

    fs::create_dir_all(dir.path().join(".specrail/agents")).unwrap();
    fs::write(
        dir.path().join(".specrail/agents/mock_existing_test_output.json"),
        r##"{
  "tests": [
    {
      "feature_id": "auth",
      "outcome_id": "outcome-1",
      "id": "auth-outcome-1-validates-credentials",
      "name": "validates_credentials",
      "path": "tests/auth/validate.rs",
      "kind": "unit",
      "purpose_refs": ["goal:Validate credentials."],
      "content": "#[test]\nfn validates_credentials() {\n    assert!(true);\n}\n"
    }
  ]
}"##,
    )
    .unwrap();

    specrail(&dir)
        .env("SPECRAIL_AGENT_CMD", "cat .specrail/agents/mock_existing_test_output.json")
        .args(["test", "generate", "--agent", "generic-shell"])
        .assert()
        .success()
        .stdout(contains("Generated 1 test file(s)"));

    let tests = support::tests(&dir);
    assert!(tests.iter().any(|test| {
        test.id == "auth-outcome-1-validates-credentials"
            && test.name == "validates_credentials"
            && test.path == "tests/auth/validate.rs"
            && test.status == "written"
    }));
}

#[test]
fn test_generate_requires_declared_required_tests() {
    let dir = TempDir::new().unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir)
        .args([
            "feature", "new", "auth",
            "--title", "Auth",
            "--purpose", "Authenticate users.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "new", "auth", "outcome-1",
            "--title", "Validation",
            "--goal", "Validate credentials.",
            "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["test", "generate", "--agent", "generic-shell"])
        .assert()
        .failure()
        .stderr(contains("no complete outcome required test declarations found"));
}

#[test]
fn test_suggest_previews_generated_tests_without_writing_files() {
    let dir = TempDir::new().unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir)
        .args([
            "feature", "new", "auth",
            "--title", "Auth",
            "--purpose", "Authenticate users.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "new", "auth", "outcome-1",
            "--title", "Validation",
            "--goal", "Validate credentials.",
            "--order", "1",
            "--test", "auth-outcome-1-validates-credentials",
            "--test-file", "tests/auth/validate.rs",
        ])
        .assert()
        .success();

    fs::create_dir_all(dir.path().join(".specrail/agents")).unwrap();
    fs::write(
        dir.path().join(".specrail/agents/mock_test_output.json"),
        r##"{
  "tests": [
    {
      "feature_id": "auth",
      "outcome_id": "outcome-1",
            "id": "auth-outcome-1-validates-credentials",
            "name": "validates_credentials",
      "path": "tests/auth/validate.rs",
      "kind": "unit",
      "purpose_refs": ["goal:Validate credentials."],
      "content": "#[test]\nfn validates_credentials() {\n    assert!(true);\n}\n"
    }
  ]
}"##,
    )
    .unwrap();

    specrail(&dir)
        .env("SPECRAIL_AGENT_CMD", "cat .specrail/agents/mock_test_output.json")
        .args([
            "test", "suggest",
            "--feature", "auth",
            "--outcome", "outcome-1",
            "--agent", "generic-shell",
        ])
        .assert()
        .success()
        .stdout(contains("Previewed 1 suggested test file(s)"))
        .stdout(contains("Preview only: no files were written"));

    assert!(!dir.path().join("tests/auth/validate.rs").exists());

    let tests = support::tests(&dir);
    assert!(tests.is_empty());
}

#[test]
fn test_add_syncs_required_test_into_outcome_yaml() {
    let dir = TempDir::new().unwrap();

    specrail(&dir).args(["init", "--no-wizard"]).assert().success();

    specrail(&dir)
        .args([
            "feature", "new", "auth",
            "--title", "Auth",
            "--purpose", "Authenticate users.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "outcome", "new", "auth", "outcome-1",
            "--title", "Validation",
            "--goal", "Validate credentials.",
            "--order", "1",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "test", "add", "auth-outcome-1-validate",
            "--feature", "auth",
            "--outcome", "outcome-1",
            "--path", "tests/auth/validate.rs",
            "--kind", "unit",
        ])
        .assert()
        .success()
        .stdout(contains("required_tests updated with auth-outcome-1-validate"))
        .stdout(contains("Status:  planned"));

    specrail(&dir)
        .args(["outcome", "show", "auth", "outcome-1"])
        .assert()
        .success()
        .stdout(contains("auth-outcome-1-validate"))
        .stdout(contains("tests/auth/validate.rs"));
}