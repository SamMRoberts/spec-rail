use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::runtime::filesystem::ensure_dir;

use super::{
    config::{ProjectConfig, ProjectSettings},
    database::Database,
    models::{
        ComponentSpec, FeatureSpec, OutcomeSpec, ProjectSpec, ProjectState, SolutionSpec,
        TestManifest,
    },
};

const SPECRAIL_DIR: &str = ".specrail";
pub const DEFAULT_SOLUTION_ID: &str = "default-solution";
pub const DEFAULT_PROJECT_ID: &str = "default-project";
pub const DEFAULT_COMPONENT_ID: &str = "default-component";

/// Central helper for resolving all `.specrail/` paths and loading/saving
/// project artifacts.
pub struct Repository {
    /// Root of the user project (directory containing `.specrail/`).
    pub root: PathBuf,
}

impl Repository {
    /// Build a [`Repository`] rooted at `root`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Attempt to locate the `.specrail/` directory by walking up from `cwd`.
    pub fn discover(cwd: impl AsRef<Path>) -> Result<Self> {
        let mut dir = cwd.as_ref().to_path_buf();
        loop {
            if dir.join(SPECRAIL_DIR).is_dir() {
                return Ok(Self::new(dir));
            }
            if !dir.pop() {
                anyhow::bail!(
                    "project not initialized — run `specrail init` first"
                );
            }
        }
    }

    // ── Storage paths ──────────────────────────────────────────────────────

    pub fn specrail_dir(&self) -> PathBuf {
        self.root.join(SPECRAIL_DIR)
    }

    pub fn state_dir(&self) -> PathBuf {
        self.specrail_dir().join("state")
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.specrail_dir().join("agents")
    }

    pub fn db_path(&self) -> PathBuf {
        self.specrail_dir().join("specrail.db")
    }

    // ── File paths ─────────────────────────────────────────────────────────

    pub fn project_config_path(&self) -> PathBuf {
        self.specrail_dir().join("project.yaml")
    }

    /// Directory that holds one `<project-id>.yaml` file per specrail project.
    pub fn projects_config_dir(&self) -> PathBuf {
        self.specrail_dir().join("projects")
    }

    /// Path to the per-project settings file for the given project ID.
    pub fn project_settings_path(&self, project_id: &str) -> PathBuf {
        self.projects_config_dir().join(format!("{project_id}.yaml"))
    }

    pub fn ledger_path(&self) -> PathBuf {
        self.state_dir().join("ledger.jsonl")
    }

    // ── Load helpers ───────────────────────────────────────────────────────

    pub fn load_config(&self) -> Result<ProjectConfig> {
        ProjectConfig::load(&self.project_config_path())
    }

    /// Load per-project settings for the given project ID.
    pub fn load_project_settings(&self, project_id: &str) -> Result<ProjectSettings> {
        ProjectSettings::load(&self.project_settings_path(project_id))
    }

    /// Persist per-project settings for the given project ID, creating the
    /// `.specrail/projects/` directory if it does not yet exist.
    pub fn save_project_settings(&self, project_id: &str, settings: &ProjectSettings) -> Result<()> {
        ensure_dir(&self.projects_config_dir())?;
        settings.save(&self.project_settings_path(project_id))
    }

    /// Return the effective [`ProjectConfig`] for the currently active project.
    ///
    /// When the active project has a per-project settings file (at
    /// `.specrail/projects/<id>.yaml`), its `test_command` and optional
    /// `default_agent` take precedence over the workspace-level
    /// `.specrail/project.yaml`.  Falls back to the workspace config when no
    /// per-project file exists.
    pub fn effective_config(&self) -> Result<ProjectConfig> {
        let state = self.load_state()?;
        if let Some(project_id) = &state.active_project {
            let settings_path = self.project_settings_path(project_id);
            if settings_path.exists() {
                if let Ok(settings) = ProjectSettings::load(&settings_path) {
                    let workspace = self.load_config().unwrap_or_default();
                    return Ok(ProjectConfig {
                        version: workspace.version,
                        name: workspace.name,
                        test_command: settings.test_command,
                        default_agent: settings
                            .default_agent
                            .unwrap_or(workspace.default_agent),
                    });
                }
            }
        }
        self.load_config()
    }

    pub fn load_state(&self) -> Result<ProjectState> {
        self.database()?.load_state()
    }

    pub fn save_state(&self, state: &ProjectState) -> Result<()> {
        self.database()?.save_state(state)
    }

    pub fn load_feature(&self, feature_id: &str) -> Result<FeatureSpec> {
        self.database()?.load_feature(feature_id)
    }

    pub fn load_solution(&self, solution_id: &str) -> Result<SolutionSpec> {
        self.database()?.load_solution(solution_id)
    }

    pub fn save_solution(&self, solution: &SolutionSpec) -> Result<()> {
        self.database()?.save_solution(solution)
    }

    pub fn load_project(&self, project_id: &str) -> Result<ProjectSpec> {
        self.database()?.load_project(project_id)
    }

    pub fn save_project(&self, project: &ProjectSpec) -> Result<()> {
        self.database()?.save_project(project)
    }

    pub fn load_component(&self, component_id: &str) -> Result<ComponentSpec> {
        self.database()?.load_component(component_id)
    }

    pub fn save_component(&self, component: &ComponentSpec) -> Result<()> {
        self.database()?.save_component(component)
    }

    pub fn save_feature(&self, feature: &FeatureSpec) -> Result<()> {
        self.database()?.save_feature(feature)
    }

    pub fn load_outcome(&self, feature_id: &str, outcome_id: &str) -> Result<OutcomeSpec> {
        self.database()?.load_outcome(feature_id, outcome_id)
    }

    pub fn save_outcome(&self, outcome: &OutcomeSpec) -> Result<()> {
        self.database()?.save_outcome(outcome)
    }

    pub fn load_manifest(&self) -> Result<TestManifest> {
        self.database()?.load_manifest()
    }

    pub fn save_manifest(&self, manifest: &TestManifest) -> Result<()> {
        self.database_mut()?.save_manifest(manifest)
    }

    pub fn ensure_hierarchy(&self) -> Result<()> {
        let database = self.database()?;

        if !database.solution_exists(DEFAULT_SOLUTION_ID)? {
            self.save_solution(&SolutionSpec {
                id: DEFAULT_SOLUTION_ID.to_string(),
                title: "Default Solution".to_string(),
                purpose: "Default solution for this repository.".to_string(),
            })?;
        }

        if !database.project_exists(DEFAULT_PROJECT_ID)? {
            self.save_project(&ProjectSpec {
                id: DEFAULT_PROJECT_ID.to_string(),
                solution_id: DEFAULT_SOLUTION_ID.to_string(),
                title: "Default Project".to_string(),
                purpose: "Default project for this repository.".to_string(),
            })?;
        }

        // Ensure the default project has its own project.yaml settings file.
        if !self.project_settings_path(DEFAULT_PROJECT_ID).exists() {
            self.save_project_settings(
                DEFAULT_PROJECT_ID,
                &ProjectSettings::for_project(&self.root),
            )?;
        }

        if !database.component_exists(DEFAULT_COMPONENT_ID)? {
            self.save_component(&ComponentSpec {
                id: DEFAULT_COMPONENT_ID.to_string(),
                solution_id: DEFAULT_SOLUTION_ID.to_string(),
                project_id: DEFAULT_PROJECT_ID.to_string(),
                title: "Default Component".to_string(),
                purpose: "Default component for this repository.".to_string(),
            })?;
        }

        let mut state = self.load_state()?;
        let mut changed = false;

        if state.active_solution.is_none() {
            state.active_solution = Some(DEFAULT_SOLUTION_ID.to_string());
            changed = true;
        }
        if state.active_project.is_none() {
            state.active_project = Some(DEFAULT_PROJECT_ID.to_string());
            changed = true;
        }
        if state.active_component.is_none() {
            state.active_component = Some(DEFAULT_COMPONENT_ID.to_string());
            changed = true;
        }

        if changed {
            self.save_state(&state)?;
        }

        Ok(())
    }

    /// List all features by reading every `*.yaml` in `.specrail/features/`.
    pub fn list_features(&self) -> Result<Vec<FeatureSpec>> {
        self.database()?.list_features()
    }

    pub fn list_features_for_component(&self, component_id: &str) -> Result<Vec<FeatureSpec>> {
        self.database()?.list_features_for_component(component_id)
    }

    pub fn list_solutions(&self) -> Result<Vec<SolutionSpec>> {
        self.database()?.list_solutions()
    }

    pub fn list_projects(&self, solution_id: &str) -> Result<Vec<ProjectSpec>> {
        self.database()?.list_projects(solution_id)
    }

    pub fn list_components(&self, project_id: &str) -> Result<Vec<ComponentSpec>> {
        self.database()?.list_components(project_id)
    }

    /// List all outcomes for a given feature, sorted by `order`.
    pub fn list_outcomes(&self, feature_id: &str) -> Result<Vec<OutcomeSpec>> {
        self.database()?.list_outcomes(feature_id)
    }

    pub fn solution_exists(&self, solution_id: &str) -> Result<bool> {
        self.database()?.solution_exists(solution_id)
    }

    pub fn project_exists(&self, project_id: &str) -> Result<bool> {
        self.database()?.project_exists(project_id)
    }

    pub fn component_exists(&self, component_id: &str) -> Result<bool> {
        self.database()?.component_exists(component_id)
    }

    pub fn feature_exists(&self, feature_id: &str) -> Result<bool> {
        self.database()?.feature_exists(feature_id)
    }

    pub fn outcome_exists(&self, feature_id: &str, outcome_id: &str) -> Result<bool> {
        self.database()?.outcome_exists(feature_id, outcome_id)
    }

    pub fn initialize_database(&self) -> Result<()> {
        self.database().map(|_| ())
    }

    fn database(&self) -> Result<Database> {
        Database::open(&self.db_path())
    }

    fn database_mut(&self) -> Result<Database> {
        Database::open(&self.db_path())
    }

    /// Return `true` when `.specrail/` exists at this repository root.
    #[allow(dead_code)]
    pub fn is_initialized(&self) -> bool {
        self.specrail_dir().is_dir()
    }
}
