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
fn feature_new_creates_file() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "feature", "new", "auth-login",
            "--title", "User Login",
            "--purpose", "Allow users to authenticate.",
        ])
        .assert()
        .success();

    assert!(
        dir.path()
            .join(".specrail/features/auth-login.yaml")
            .exists()
    );
}

#[test]
fn feature_new_records_ledger_event() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "feature", "new", "auth-login",
            "--title", "User Login",
            "--purpose", "Allow users to authenticate.",
        ])
        .assert()
        .success();

    let ledger =
        fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(ledger.contains("feature_created"));
    assert!(ledger.contains("auth-login"));
}

#[test]
fn feature_list_shows_created_feature() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "feature", "new", "my-feature",
            "--title", "My Feature",
            "--purpose", "Do something useful.",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["feature", "list"])
        .assert()
        .success()
        .stdout(contains("my-feature"));
}

#[test]
fn feature_show_displays_details() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "feature", "new", "search",
            "--title", "Product Search",
            "--purpose", "Users can search products.",
            "--outcome", "Results appear within 200ms",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["feature", "show", "search"])
        .assert()
        .success()
        .stdout(contains("Product Search"))
        .stdout(contains("Users can search products."))
        .stdout(contains("Results appear within 200ms"));
}

#[test]
fn feature_new_fails_on_duplicate() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "feature", "new", "dup",
            "--title", "Dup", "--purpose", "p",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "feature", "new", "dup",
            "--title", "Dup Again", "--purpose", "p",
        ])
        .assert()
        .failure();
}

#[test]
fn feature_activate_updates_state() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "feature", "new", "feat",
            "--title", "Feat", "--purpose", "p",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args(["feature", "activate", "feat"])
        .assert()
        .success();

    let state =
        fs::read_to_string(dir.path().join(".specrail/state/current.yaml")).unwrap();
    assert!(state.contains("feat"), "state should reference active feature");
}

#[test]
fn feature_edit_updates_existing_file() {
    let dir = TempDir::new().unwrap();
    init(&dir);

    specrail(&dir)
        .args([
            "feature", "new", "search",
            "--title", "Search",
            "--purpose", "Initial purpose.",
            "--outcome", "Original outcome",
        ])
        .assert()
        .success();

    specrail(&dir)
        .args([
            "feature", "edit", "search",
            "--title", "Advanced Search",
            "--purpose", "Updated purpose.",
            "--outcome", "Fast query results",
            "--constraint", "Stay under 200ms",
        ])
        .assert()
        .success()
        .stdout(contains("updated"));

    let feature = fs::read_to_string(dir.path().join(".specrail/features/search.yaml")).unwrap();
    assert!(feature.contains("Advanced Search"));
    assert!(feature.contains("Updated purpose."));
    assert!(feature.contains("Fast query results"));
    assert!(feature.contains("Stay under 200ms"));

    let ledger = fs::read_to_string(dir.path().join(".specrail/state/ledger.jsonl")).unwrap();
    assert!(ledger.contains("feature_edited"));
}
