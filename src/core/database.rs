use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

use super::models::{
    ComponentSpec, FeatureSpec, OutcomeSpec, ProjectSpec, ProjectState, SolutionSpec,
    TestManifest, TestSpec,
};

const SCHEMA_VERSION: i64 = 3;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating database directory {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("opening database {}", path.display()))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("enabling sqlite foreign keys")?;
        Self::initialize_schema(&conn)?;

        Ok(Self { conn })
    }

    fn initialize_schema(conn: &Connection) -> Result<()> {
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .context("reading sqlite schema version")?;

        if version > SCHEMA_VERSION {
            bail!("unsupported specrail database schema version {version}");
        }

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS solutions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                purpose TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                solution_id TEXT NOT NULL,
                title TEXT NOT NULL,
                purpose TEXT NOT NULL,
                FOREIGN KEY(solution_id) REFERENCES solutions(id) ON DELETE RESTRICT
            );

            CREATE TABLE IF NOT EXISTS components (
                id TEXT PRIMARY KEY,
                solution_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                title TEXT NOT NULL,
                purpose TEXT NOT NULL,
                FOREIGN KEY(solution_id) REFERENCES solutions(id) ON DELETE RESTRICT,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT
            );

            CREATE TABLE IF NOT EXISTS features (
                id TEXT PRIMARY KEY,
                solution_id TEXT NOT NULL,
                project_id TEXT NOT NULL,
                component_id TEXT NOT NULL,
                title TEXT NOT NULL,
                purpose TEXT NOT NULL,
                outcomes_json TEXT NOT NULL,
                constraints_json TEXT NOT NULL,
                non_goals_json TEXT NOT NULL,
                dependencies_json TEXT NOT NULL,
                status TEXT NOT NULL,
                current_outcome TEXT,
                FOREIGN KEY(solution_id) REFERENCES solutions(id) ON DELETE RESTRICT,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE RESTRICT,
                FOREIGN KEY(component_id) REFERENCES components(id) ON DELETE RESTRICT
            );

            CREATE TABLE IF NOT EXISTS outcomes (
                feature_id TEXT NOT NULL,
                id TEXT NOT NULL,
                title TEXT NOT NULL,
                goal TEXT NOT NULL,
                order_index INTEGER NOT NULL,
                prerequisites_json TEXT NOT NULL,
                allowed_paths_json TEXT NOT NULL,
                forbidden_paths_json TEXT NOT NULL,
                required_tests_json TEXT NOT NULL,
                required_test_files_json TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL,
                PRIMARY KEY(feature_id, id),
                FOREIGN KEY(feature_id) REFERENCES features(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS tests (
                id TEXT PRIMARY KEY,
                feature_id TEXT NOT NULL,
                outcome_id TEXT NOT NULL,
                path TEXT NOT NULL,
                purpose_refs_json TEXT NOT NULL,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                FOREIGN KEY(feature_id) REFERENCES features(id) ON DELETE CASCADE,
                FOREIGN KEY(feature_id, outcome_id) REFERENCES outcomes(feature_id, id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS current_state (
                slot INTEGER PRIMARY KEY CHECK(slot = 1),
                active_solution TEXT,
                active_project TEXT,
                active_component TEXT,
                active_feature TEXT,
                active_outcome TEXT
            );
            ",
        )
        .context("initializing sqlite schema")?;

        if version > 0 && version < 3 {
            conn.execute_batch(
                "
                ALTER TABLE outcomes ADD COLUMN required_test_files_json TEXT NOT NULL DEFAULT '[]';
                UPDATE outcomes
                SET required_test_files_json = required_tests_json,
                    required_tests_json = '[]'
                WHERE required_test_files_json = '[]';
                ",
            )
            .context("migrating outcomes required test metadata")?;
        }

        if version < SCHEMA_VERSION {
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)
                .context("writing sqlite schema version")?;
        }

        Ok(())
    }

    pub fn solution_exists(&self, id: &str) -> Result<bool> {
        self.exists("SELECT 1 FROM solutions WHERE id = ?1", params![id])
    }

    pub fn project_exists(&self, id: &str) -> Result<bool> {
        self.exists("SELECT 1 FROM projects WHERE id = ?1", params![id])
    }

    pub fn component_exists(&self, id: &str) -> Result<bool> {
        self.exists("SELECT 1 FROM components WHERE id = ?1", params![id])
    }

    pub fn feature_exists(&self, id: &str) -> Result<bool> {
        self.exists("SELECT 1 FROM features WHERE id = ?1", params![id])
    }

    pub fn outcome_exists(&self, feature_id: &str, outcome_id: &str) -> Result<bool> {
        self.exists(
            "SELECT 1 FROM outcomes WHERE feature_id = ?1 AND id = ?2",
            params![feature_id, outcome_id],
        )
    }

    pub fn load_solution(&self, id: &str) -> Result<SolutionSpec> {
        self.load_solution_opt(id)?
            .with_context(|| format!("solution '{id}' not found"))
    }

    pub fn save_solution(&self, solution: &SolutionSpec) -> Result<()> {
        self.conn
            .execute(
                "
                INSERT INTO solutions (id, title, purpose)
                VALUES (?1, ?2, ?3)
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    purpose = excluded.purpose
                ",
                params![solution.id, solution.title, solution.purpose],
            )
            .with_context(|| format!("saving solution '{}'", solution.id))?;
        Ok(())
    }

    pub fn list_solutions(&self) -> Result<Vec<SolutionSpec>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, title, purpose FROM solutions ORDER BY id")
            .context("preparing solutions query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(SolutionSpec {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    purpose: row.get(2)?,
                })
            })
            .context("querying solutions")?;
        collect_rows(rows)
    }

    pub fn load_project(&self, id: &str) -> Result<ProjectSpec> {
        self.load_project_opt(id)?
            .with_context(|| format!("project '{id}' not found"))
    }

    pub fn save_project(&self, project: &ProjectSpec) -> Result<()> {
        self.conn
            .execute(
                "
                INSERT INTO projects (id, solution_id, title, purpose)
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(id) DO UPDATE SET
                    solution_id = excluded.solution_id,
                    title = excluded.title,
                    purpose = excluded.purpose
                ",
                params![project.id, project.solution_id, project.title, project.purpose],
            )
            .with_context(|| format!("saving project '{}'", project.id))?;
        Ok(())
    }

    pub fn list_projects(&self, solution_id: &str) -> Result<Vec<ProjectSpec>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, solution_id, title, purpose FROM projects WHERE solution_id = ?1 ORDER BY id",
            )
            .context("preparing projects query")?;
        let rows = stmt
            .query_map(params![solution_id], |row| {
                Ok(ProjectSpec {
                    id: row.get(0)?,
                    solution_id: row.get(1)?,
                    title: row.get(2)?,
                    purpose: row.get(3)?,
                })
            })
            .with_context(|| format!("querying projects for solution '{solution_id}'"))?;
        collect_rows(rows)
    }

    pub fn load_component(&self, id: &str) -> Result<ComponentSpec> {
        self.load_component_opt(id)?
            .with_context(|| format!("component '{id}' not found"))
    }

    pub fn save_component(&self, component: &ComponentSpec) -> Result<()> {
        self.conn
            .execute(
                "
                INSERT INTO components (id, solution_id, project_id, title, purpose)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(id) DO UPDATE SET
                    solution_id = excluded.solution_id,
                    project_id = excluded.project_id,
                    title = excluded.title,
                    purpose = excluded.purpose
                ",
                params![
                    component.id,
                    component.solution_id,
                    component.project_id,
                    component.title,
                    component.purpose
                ],
            )
            .with_context(|| format!("saving component '{}'", component.id))?;
        Ok(())
    }

    pub fn list_components(&self, project_id: &str) -> Result<Vec<ComponentSpec>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, solution_id, project_id, title, purpose FROM components WHERE project_id = ?1 ORDER BY id",
            )
            .context("preparing components query")?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ComponentSpec {
                    id: row.get(0)?,
                    solution_id: row.get(1)?,
                    project_id: row.get(2)?,
                    title: row.get(3)?,
                    purpose: row.get(4)?,
                })
            })
            .with_context(|| format!("querying components for project '{project_id}'"))?;
        collect_rows(rows)
    }

    pub fn load_feature(&self, id: &str) -> Result<FeatureSpec> {
        self.load_feature_opt(id)?
            .with_context(|| format!("feature '{id}' not found"))
    }

    pub fn save_feature(&self, feature: &FeatureSpec) -> Result<()> {
        self.conn
            .execute(
                "
                INSERT INTO features (
                    id, solution_id, project_id, component_id, title, purpose,
                    outcomes_json, constraints_json, non_goals_json, dependencies_json,
                    status, current_outcome
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ON CONFLICT(id) DO UPDATE SET
                    solution_id = excluded.solution_id,
                    project_id = excluded.project_id,
                    component_id = excluded.component_id,
                    title = excluded.title,
                    purpose = excluded.purpose,
                    outcomes_json = excluded.outcomes_json,
                    constraints_json = excluded.constraints_json,
                    non_goals_json = excluded.non_goals_json,
                    dependencies_json = excluded.dependencies_json,
                    status = excluded.status,
                    current_outcome = excluded.current_outcome
                ",
                params![
                    feature.id,
                    feature.solution_id,
                    feature.project_id,
                    feature.component_id,
                    feature.title,
                    feature.purpose,
                    encode_json(&feature.outcomes)?,
                    encode_json(&feature.constraints)?,
                    encode_json(&feature.non_goals)?,
                    encode_json(&feature.dependencies)?,
                    encode_enum(&feature.status)?,
                    feature.current_outcome,
                ],
            )
            .with_context(|| format!("saving feature '{}'", feature.id))?;
        Ok(())
    }

    pub fn list_features(&self) -> Result<Vec<FeatureSpec>> {
        let mut stmt = self
            .conn
            .prepare(
                "
                SELECT id, solution_id, project_id, component_id, title, purpose,
                       outcomes_json, constraints_json, non_goals_json, dependencies_json,
                       status, current_outcome
                FROM features
                ORDER BY id
                ",
            )
            .context("preparing features query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(FeatureRow {
                    id: row.get(0)?,
                    solution_id: row.get(1)?,
                    project_id: row.get(2)?,
                    component_id: row.get(3)?,
                    title: row.get(4)?,
                    purpose: row.get(5)?,
                    outcomes_json: row.get(6)?,
                    constraints_json: row.get(7)?,
                    non_goals_json: row.get(8)?,
                    dependencies_json: row.get(9)?,
                    status: row.get(10)?,
                    current_outcome: row.get(11)?,
                })
            })
            .context("querying features")?;
        collect_feature_rows(rows)
    }

    pub fn list_features_for_component(&self, component_id: &str) -> Result<Vec<FeatureSpec>> {
        let mut stmt = self
            .conn
            .prepare(
                "
                SELECT id, solution_id, project_id, component_id, title, purpose,
                       outcomes_json, constraints_json, non_goals_json, dependencies_json,
                       status, current_outcome
                FROM features
                WHERE component_id = ?1
                ORDER BY id
                ",
            )
            .context("preparing component feature query")?;
        let rows = stmt
            .query_map(params![component_id], |row| {
                Ok(FeatureRow {
                    id: row.get(0)?,
                    solution_id: row.get(1)?,
                    project_id: row.get(2)?,
                    component_id: row.get(3)?,
                    title: row.get(4)?,
                    purpose: row.get(5)?,
                    outcomes_json: row.get(6)?,
                    constraints_json: row.get(7)?,
                    non_goals_json: row.get(8)?,
                    dependencies_json: row.get(9)?,
                    status: row.get(10)?,
                    current_outcome: row.get(11)?,
                })
            })
            .with_context(|| format!("querying features for component '{component_id}'"))?;
        collect_feature_rows(rows)
    }

    pub fn load_outcome(&self, feature_id: &str, outcome_id: &str) -> Result<OutcomeSpec> {
        self.load_outcome_opt(feature_id, outcome_id)?
            .with_context(|| format!("outcome '{outcome_id}' not found for feature '{feature_id}'"))
    }

    pub fn save_outcome(&self, outcome: &OutcomeSpec) -> Result<()> {
        self.conn
            .execute(
                "
                INSERT INTO outcomes (
                    feature_id, id, title, goal, order_index,
                    prerequisites_json, allowed_paths_json, forbidden_paths_json,
                    required_tests_json, required_test_files_json, status
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(feature_id, id) DO UPDATE SET
                    title = excluded.title,
                    goal = excluded.goal,
                    order_index = excluded.order_index,
                    prerequisites_json = excluded.prerequisites_json,
                    allowed_paths_json = excluded.allowed_paths_json,
                    forbidden_paths_json = excluded.forbidden_paths_json,
                    required_tests_json = excluded.required_tests_json,
                    required_test_files_json = excluded.required_test_files_json,
                    status = excluded.status
                ",
                params![
                    outcome.feature_id,
                    outcome.id,
                    outcome.title,
                    outcome.goal,
                    outcome.order,
                    encode_json(&outcome.prerequisites)?,
                    encode_json(&outcome.allowed_paths)?,
                    encode_json(&outcome.forbidden_paths)?,
                    encode_json(&outcome.required_tests)?,
                    encode_json(&outcome.required_test_files)?,
                    encode_enum(&outcome.status)?,
                ],
            )
            .with_context(|| format!("saving outcome '{}:{}'", outcome.feature_id, outcome.id))?;
        Ok(())
    }

    pub fn load_manifest(&self) -> Result<TestManifest> {
        let mut stmt = self
            .conn
            .prepare(
                "
                SELECT id, feature_id, outcome_id, path, purpose_refs_json, kind, status
                FROM tests
                ORDER BY id
                ",
            )
            .context("preparing tests query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(TestRow {
                    id: row.get(0)?,
                    feature_id: row.get(1)?,
                    outcome_id: row.get(2)?,
                    path: row.get(3)?,
                    purpose_refs_json: row.get(4)?,
                    kind: row.get(5)?,
                    status: row.get(6)?,
                })
            })
            .context("querying tests")?;

        let mut tests = Vec::new();
        for row in rows {
            tests.push(TryInto::<TestSpec>::try_into(row.context("reading test row")?)?);
        }

        Ok(TestManifest { tests })
    }

    pub fn save_manifest(&mut self, manifest: &TestManifest) -> Result<()> {
        let tx = self.conn.transaction().context("starting tests transaction")?;
        tx.execute("DELETE FROM tests", [])
            .context("clearing tests table")?;

        for test in &manifest.tests {
            tx.execute(
                "
                INSERT INTO tests (id, feature_id, outcome_id, path, purpose_refs_json, kind, status)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ",
                params![
                    test.id,
                    test.feature_id,
                    test.outcome_id,
                    test.path,
                    encode_json(&test.purpose_refs)?,
                    encode_enum(&test.kind)?,
                    encode_enum(&test.status)?,
                ],
            )
            .with_context(|| format!("saving test '{}'", test.id))?;
        }

        tx.commit().context("committing tests transaction")?;
        Ok(())
    }

    pub fn load_state(&self) -> Result<ProjectState> {
        let row = self
            .conn
            .query_row(
                "
                SELECT active_solution, active_project, active_component, active_feature, active_outcome
                FROM current_state
                WHERE slot = 1
                ",
                [],
                |row| {
                    Ok(ProjectState {
                        active_solution: row.get(0)?,
                        active_project: row.get(1)?,
                        active_component: row.get(2)?,
                        active_feature: row.get(3)?,
                        active_outcome: row.get(4)?,
                    })
                },
            )
            .optional()
            .context("loading current state")?;

        Ok(row.unwrap_or_default())
    }

    pub fn save_state(&self, state: &ProjectState) -> Result<()> {
        self.conn
            .execute(
                "
                INSERT INTO current_state (
                    slot, active_solution, active_project, active_component, active_feature, active_outcome
                )
                VALUES (1, ?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(slot) DO UPDATE SET
                    active_solution = excluded.active_solution,
                    active_project = excluded.active_project,
                    active_component = excluded.active_component,
                    active_feature = excluded.active_feature,
                    active_outcome = excluded.active_outcome
                ",
                params![
                    state.active_solution,
                    state.active_project,
                    state.active_component,
                    state.active_feature,
                    state.active_outcome,
                ],
            )
            .context("saving current state")?;
        Ok(())
    }

    pub fn list_outcomes(&self, feature_id: &str) -> Result<Vec<OutcomeSpec>> {
        let mut stmt = self
            .conn
            .prepare(
                "
                SELECT feature_id, id, title, goal, order_index,
                       prerequisites_json, allowed_paths_json, forbidden_paths_json,
                      required_tests_json, required_test_files_json, status
                FROM outcomes
                WHERE feature_id = ?1
                ORDER BY order_index, id
                ",
            )
            .context("preparing outcomes query")?;
        let rows = stmt
            .query_map(params![feature_id], |row| {
                Ok(OutcomeRow {
                    feature_id: row.get(0)?,
                    id: row.get(1)?,
                    title: row.get(2)?,
                    goal: row.get(3)?,
                    order: row.get(4)?,
                    prerequisites_json: row.get(5)?,
                    allowed_paths_json: row.get(6)?,
                    forbidden_paths_json: row.get(7)?,
                    required_tests_json: row.get(8)?,
                    required_test_files_json: row.get(9)?,
                    status: row.get(10)?,
                })
            })
            .with_context(|| format!("querying outcomes for feature '{feature_id}'"))?;
        collect_outcome_rows(rows)
    }

    fn load_solution_opt(&self, id: &str) -> Result<Option<SolutionSpec>> {
        self.conn
            .query_row(
                "SELECT id, title, purpose FROM solutions WHERE id = ?1",
                params![id],
                |row| {
                    Ok(SolutionSpec {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        purpose: row.get(2)?,
                    })
                },
            )
            .optional()
            .with_context(|| format!("loading solution '{id}'"))
    }

    fn load_project_opt(&self, id: &str) -> Result<Option<ProjectSpec>> {
        self.conn
            .query_row(
                "SELECT id, solution_id, title, purpose FROM projects WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ProjectSpec {
                        id: row.get(0)?,
                        solution_id: row.get(1)?,
                        title: row.get(2)?,
                        purpose: row.get(3)?,
                    })
                },
            )
            .optional()
            .with_context(|| format!("loading project '{id}'"))
    }

    fn load_component_opt(&self, id: &str) -> Result<Option<ComponentSpec>> {
        self.conn
            .query_row(
                "SELECT id, solution_id, project_id, title, purpose FROM components WHERE id = ?1",
                params![id],
                |row| {
                    Ok(ComponentSpec {
                        id: row.get(0)?,
                        solution_id: row.get(1)?,
                        project_id: row.get(2)?,
                        title: row.get(3)?,
                        purpose: row.get(4)?,
                    })
                },
            )
            .optional()
            .with_context(|| format!("loading component '{id}'"))
    }

    fn load_feature_opt(&self, id: &str) -> Result<Option<FeatureSpec>> {
        let row = self
            .conn
            .query_row(
                "
                SELECT id, solution_id, project_id, component_id, title, purpose,
                       outcomes_json, constraints_json, non_goals_json, dependencies_json,
                       status, current_outcome
                FROM features
                WHERE id = ?1
                ",
                params![id],
                |row| {
                    Ok(FeatureRow {
                        id: row.get(0)?,
                        solution_id: row.get(1)?,
                        project_id: row.get(2)?,
                        component_id: row.get(3)?,
                        title: row.get(4)?,
                        purpose: row.get(5)?,
                        outcomes_json: row.get(6)?,
                        constraints_json: row.get(7)?,
                        non_goals_json: row.get(8)?,
                        dependencies_json: row.get(9)?,
                        status: row.get(10)?,
                        current_outcome: row.get(11)?,
                    })
                },
            )
            .optional()
            .with_context(|| format!("loading feature '{id}'"))?;

        row.map(TryInto::try_into).transpose()
    }

    fn load_outcome_opt(&self, feature_id: &str, outcome_id: &str) -> Result<Option<OutcomeSpec>> {
        let row = self
            .conn
            .query_row(
                "
                SELECT feature_id, id, title, goal, order_index,
                       prerequisites_json, allowed_paths_json, forbidden_paths_json,
                      required_tests_json, required_test_files_json, status
                FROM outcomes
                WHERE feature_id = ?1 AND id = ?2
                ",
                params![feature_id, outcome_id],
                |row| {
                    Ok(OutcomeRow {
                        feature_id: row.get(0)?,
                        id: row.get(1)?,
                        title: row.get(2)?,
                        goal: row.get(3)?,
                        order: row.get(4)?,
                        prerequisites_json: row.get(5)?,
                        allowed_paths_json: row.get(6)?,
                        forbidden_paths_json: row.get(7)?,
                        required_tests_json: row.get(8)?,
                        required_test_files_json: row.get(9)?,
                        status: row.get(10)?,
                    })
                },
            )
            .optional()
            .with_context(|| format!("loading outcome '{feature_id}:{outcome_id}'"))?;

        row.map(TryInto::try_into).transpose()
    }

    fn exists<P>(&self, sql: &str, params: P) -> Result<bool>
    where
        P: rusqlite::Params,
    {
        let found = self
            .conn
            .query_row(sql, params, |_| Ok(()))
            .optional()
            .context("checking row existence")?;
        Ok(found.is_some())
    }
}

struct FeatureRow {
    id: String,
    solution_id: String,
    project_id: String,
    component_id: String,
    title: String,
    purpose: String,
    outcomes_json: String,
    constraints_json: String,
    non_goals_json: String,
    dependencies_json: String,
    status: String,
    current_outcome: Option<String>,
}

impl TryFrom<FeatureRow> for FeatureSpec {
    type Error = anyhow::Error;

    fn try_from(value: FeatureRow) -> Result<Self> {
        Ok(Self {
            id: value.id,
            solution_id: value.solution_id,
            project_id: value.project_id,
            component_id: value.component_id,
            title: value.title,
            purpose: value.purpose,
            outcomes: decode_json(&value.outcomes_json)?,
            constraints: decode_json(&value.constraints_json)?,
            non_goals: decode_json(&value.non_goals_json)?,
            dependencies: decode_json(&value.dependencies_json)?,
            status: decode_enum(&value.status)?,
            current_outcome: value.current_outcome,
        })
    }
}

struct OutcomeRow {
    feature_id: String,
    id: String,
    title: String,
    goal: String,
    order: u32,
    prerequisites_json: String,
    allowed_paths_json: String,
    forbidden_paths_json: String,
    required_tests_json: String,
    required_test_files_json: String,
    status: String,
}

struct TestRow {
    id: String,
    feature_id: String,
    outcome_id: String,
    path: String,
    purpose_refs_json: String,
    kind: String,
    status: String,
}

impl TryFrom<TestRow> for TestSpec {
    type Error = anyhow::Error;

    fn try_from(value: TestRow) -> Result<Self> {
        Ok(Self {
            id: value.id,
            feature_id: value.feature_id,
            outcome_id: value.outcome_id,
            path: value.path,
            purpose_refs: decode_json(&value.purpose_refs_json)?,
            kind: decode_enum(&value.kind)?,
            status: decode_enum(&value.status)?,
        })
    }
}

impl TryFrom<OutcomeRow> for OutcomeSpec {
    type Error = anyhow::Error;

    fn try_from(value: OutcomeRow) -> Result<Self> {
        Ok(Self {
            id: value.id,
            feature_id: value.feature_id,
            title: value.title,
            goal: value.goal,
            order: value.order,
            prerequisites: decode_json(&value.prerequisites_json)?,
            allowed_paths: decode_json(&value.allowed_paths_json)?,
            forbidden_paths: decode_json(&value.forbidden_paths_json)?,
            required_tests: decode_json(&value.required_tests_json)?,
            required_test_files: decode_json(&value.required_test_files_json)?,
            status: decode_enum(&value.status)?,
        })
    }
}

fn collect_rows<T>(rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>>) -> Result<Vec<T>> {
    let mut items = Vec::new();
    for row in rows {
        items.push(row.context("reading sqlite row")?);
    }
    Ok(items)
}

fn collect_feature_rows(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<FeatureRow>>,
) -> Result<Vec<FeatureSpec>> {
    let mut items = Vec::new();
    for row in rows {
        items.push(TryInto::<FeatureSpec>::try_into(row.context("reading feature row")?)?);
    }
    Ok(items)
}

fn collect_outcome_rows(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<OutcomeRow>>,
) -> Result<Vec<OutcomeSpec>> {
    let mut items = Vec::new();
    for row in rows {
        items.push(TryInto::<OutcomeSpec>::try_into(row.context("reading outcome row")?)?);
    }
    Ok(items)
}

fn encode_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).context("serializing json field")
}

fn decode_json<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_str(value).with_context(|| format!("parsing json field '{value}'"))
}

fn encode_enum<T: Serialize>(value: &T) -> Result<String> {
    let json = serde_json::to_value(value).context("serializing enum value")?;
    json.as_str()
        .map(ToOwned::to_owned)
        .context("enum did not serialize to a string")
}

fn decode_enum<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .with_context(|| format!("parsing enum value '{value}'"))
}