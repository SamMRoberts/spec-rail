#![allow(dead_code)]

use rusqlite::{params, Connection};
use tempfile::TempDir;

#[derive(Debug)]
pub struct StateSnapshot {
    pub active_solution: Option<String>,
    pub active_project: Option<String>,
    pub active_component: Option<String>,
    pub active_feature: Option<String>,
    pub active_outcome: Option<String>,
}

#[derive(Debug)]
pub struct TestRecord {
    pub id: String,
    pub feature_id: String,
    pub outcome_id: String,
    pub path: String,
    pub kind: String,
    pub status: String,
}

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
    test_file: &str,
) {
    let conn = open_db(dir);
    let json: String = conn
        .query_row(
            "SELECT required_test_files_json FROM outcomes WHERE feature_id = ?1 AND id = ?2",
            params![feature_id, outcome_id],
            |row| row.get(0),
        )
        .unwrap();
    let mut tests: Vec<String> = serde_json::from_str(&json).unwrap();
    tests.retain(|candidate| candidate != test_file);
    conn.execute(
        "UPDATE outcomes SET required_test_files_json = ?1 WHERE feature_id = ?2 AND id = ?3",
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

pub fn outcome_required_test_files(
    dir: &TempDir,
    feature_id: &str,
    outcome_id: &str,
) -> Vec<String> {
    let conn = open_db(dir);
    let json: String = conn
        .query_row(
            "SELECT required_test_files_json FROM outcomes WHERE feature_id = ?1 AND id = ?2",
            params![feature_id, outcome_id],
            |row| row.get(0),
        )
        .unwrap();
    serde_json::from_str(&json).unwrap()
}

pub fn current_state(dir: &TempDir) -> StateSnapshot {
    let conn = open_db(dir);
    conn.query_row(
        "
        SELECT active_solution, active_project, active_component, active_feature, active_outcome
        FROM current_state
        WHERE slot = 1
        ",
        [],
        |row| {
            Ok(StateSnapshot {
                active_solution: row.get(0)?,
                active_project: row.get(1)?,
                active_component: row.get(2)?,
                active_feature: row.get(3)?,
                active_outcome: row.get(4)?,
            })
        },
    )
    .unwrap()
}

pub fn tests(dir: &TempDir) -> Vec<TestRecord> {
    let conn = open_db(dir);
    let mut stmt = conn
        .prepare(
            "SELECT id, feature_id, outcome_id, path, kind, status FROM tests ORDER BY id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok(TestRecord {
                id: row.get(0)?,
                feature_id: row.get(1)?,
                outcome_id: row.get(2)?,
                path: row.get(3)?,
                kind: row.get(4)?,
                status: row.get(5)?,
            })
        })
        .unwrap();

    rows.map(|row| row.unwrap()).collect()
}