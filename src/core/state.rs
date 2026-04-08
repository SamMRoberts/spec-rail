use anyhow::{Context, Result};
use std::path::Path;

use super::models::ProjectState;

#[allow(dead_code)]
impl ProjectState {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading state from {}", path.display()))?;
        let state: Self = serde_yaml::from_str(&content)
            .with_context(|| "parsing current.yaml")?;
        Ok(state)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)
            .with_context(|| format!("writing state to {}", path.display()))?;
        Ok(())
    }
}
