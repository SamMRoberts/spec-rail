#![allow(dead_code)]

use rusqlite::{params, Connection};
use tempfile::TempDir;

fn open_db(dir: &TempDir) -> Connection {
    Connection::open(dir.path().join(".specrail/specrail.db")).unwrap()
}

pub fn set_outcome_status(dir: &TempDir, feature_id: &str, outcome_id: &str, status: &str) {
    let conn = open_db(dir);
    conn.execute(
        "UPDATE outcomes SET status = ?1 WHERE feature_id = ?2 AND id = ?3",
        params![status, feature_id, outcome_id],
    )
    .unwrap();
}

pub fn remove_required_test(
    dir: &TempDir,
    feature_id: &str,
    outcome_id: &str,
    path: &str,
) {
    let conn = open_db(dir);
    let json: String = conn
        .query_row(
            "SELECT required_tests_json FROM outcomes WHERE feature_id = ?1 AND id = ?2",
            params![feature_id, outcome_id],
            |row| row.get(0),
        )
        .unwrap();
    let mut tests: Vec<String> = serde_json::from_str(&json).unwrap();
    tests.retain(|candidate| candidate != path);
    conn.execute(
        "UPDATE outcomes SET required_tests_json = ?1 WHERE feature_id = ?2 AND id = ?3",
        params![serde_json::to_string(&tests).unwrap(), feature_id, outcome_id],
    )
    .unwrap();
}

pub fn outcome_required_tests(dir: &TempDir, feature_id: &str, outcome_id: &str) -> Vec<String> {
    let conn = open_db(dir);
    let json: String = conn
        .query_row(
            "SELECT required_tests_json FROM outcomes WHERE feature_id = ?1 AND id = ?2",
            params![feature_id, outcome_id],
            |row| row.get(0),
        )
        .unwrap();
    serde_json::from_str(&json).unwrap()
}