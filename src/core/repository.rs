use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use super::{
    config::ProjectConfig,
    models::{FeatureSpec, OutcomeSpec, ProjectState, TestManifest},
};

const SPECRAIL_DIR: &str = ".specrail";

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

    // ── Directory paths ────────────────────────────────────────────────────

    pub fn specrail_dir(&self) -> PathBuf {
        self.root.join(SPECRAIL_DIR)
    }

    pub fn features_dir(&self) -> PathBuf {
        self.specrail_dir().join("features")
    }

    pub fn outcomes_dir(&self) -> PathBuf {
        self.specrail_dir().join("outcomes")
    }

    pub fn tests_dir(&self) -> PathBuf {
        self.specrail_dir().join("tests")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.specrail_dir().join("state")
    }

    pub fn agents_dir(&self) -> PathBuf {
        self.specrail_dir().join("agents")
    }

    // ── File paths ─────────────────────────────────────────────────────────

    pub fn project_config_path(&self) -> PathBuf {
        self.specrail_dir().join("project.yaml")
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.tests_dir().join("manifest.yaml")
    }

    pub fn state_path(&self) -> PathBuf {
        self.state_dir().join("current.yaml")
    }

    pub fn ledger_path(&self) -> PathBuf {
        self.state_dir().join("ledger.jsonl")
    }

    pub fn feature_path(&self, feature_id: &str) -> PathBuf {
        self.features_dir().join(format!("{feature_id}.yaml"))
    }

    pub fn outcome_path(&self, feature_id: &str, outcome_id: &str) -> PathBuf {
        self.outcomes_dir()
            .join(feature_id)
            .join(format!("{outcome_id}.yaml"))
    }

    pub fn feature_outcomes_dir(&self, feature_id: &str) -> PathBuf {
        self.outcomes_dir().join(feature_id)
    }

    // ── Load helpers ───────────────────────────────────────────────────────

    pub fn load_config(&self) -> Result<ProjectConfig> {
        ProjectConfig::load(&self.project_config_path())
    }

    pub fn load_state(&self) -> Result<ProjectState> {
        ProjectState::load(&self.state_path())
    }

    pub fn save_state(&self, state: &ProjectState) -> Result<()> {
        state.save(&self.state_path())
    }

    pub fn load_feature(&self, feature_id: &str) -> Result<FeatureSpec> {
        let path = self.feature_path(feature_id);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading feature '{feature_id}' from {}", path.display()))?;
        serde_yaml::from_str(&content)
            .with_context(|| format!("parsing feature file {}", path.display()))
    }

    pub fn save_feature(&self, feature: &FeatureSpec) -> Result<()> {
        let path = self.feature_path(&feature.id);
        let content = serde_yaml::to_string(feature)?;
        std::fs::write(&path, content)
            .with_context(|| format!("writing feature to {}", path.display()))
    }

    pub fn load_outcome(&self, feature_id: &str, outcome_id: &str) -> Result<OutcomeSpec> {
        let path = self.outcome_path(feature_id, outcome_id);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading outcome '{outcome_id}' from {}", path.display()))?;
        serde_yaml::from_str(&content)
            .with_context(|| format!("parsing outcome file {}", path.display()))
    }

    pub fn save_outcome(&self, outcome: &OutcomeSpec) -> Result<()> {
        let path = self.outcome_path(&outcome.feature_id, &outcome.id);
        std::fs::create_dir_all(path.parent().unwrap())?;
        let content = serde_yaml::to_string(outcome)?;
        std::fs::write(&path, content)
            .with_context(|| format!("writing outcome to {}", path.display()))
    }

    pub fn load_manifest(&self) -> Result<TestManifest> {
        let path = self.manifest_path();
        if !path.exists() {
            return Ok(TestManifest::default());
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading manifest from {}", path.display()))?;
        serde_yaml::from_str(&content)
            .with_context(|| format!("parsing manifest {}", path.display()))
    }

    pub fn save_manifest(&self, manifest: &TestManifest) -> Result<()> {
        let path = self.manifest_path();
        let content = serde_yaml::to_string(manifest)?;
        std::fs::write(&path, content)
            .with_context(|| format!("writing manifest to {}", path.display()))
    }

    /// List all features by reading every `*.yaml` in `.specrail/features/`.
    pub fn list_features(&self) -> Result<Vec<FeatureSpec>> {
        let dir = self.features_dir();
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut features = Vec::new();
        for entry in walkdir::WalkDir::new(&dir).max_depth(1).min_depth(1) {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                let content = std::fs::read_to_string(path)?;
                if let Ok(f) = serde_yaml::from_str::<FeatureSpec>(&content) {
                    features.push(f);
                }
            }
        }
        features.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(features)
    }

    /// List all outcomes for a given feature, sorted by `order`.
    pub fn list_outcomes(&self, feature_id: &str) -> Result<Vec<OutcomeSpec>> {
        let dir = self.feature_outcomes_dir(feature_id);
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut outcomes = Vec::new();
        for entry in walkdir::WalkDir::new(&dir).max_depth(1).min_depth(1) {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                let content = std::fs::read_to_string(path)?;
                if let Ok(o) = serde_yaml::from_str::<OutcomeSpec>(&content) {
                    outcomes.push(o);
                }
            }
        }
        outcomes.sort_by_key(|o| o.order);
        Ok(outcomes)
    }

    /// Return `true` when `.specrail/` exists at this repository root.
    #[allow(dead_code)]
    pub fn is_initialized(&self) -> bool {
        self.specrail_dir().is_dir()
    }
}
