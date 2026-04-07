use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectConfig {
    pub version: u32,
    pub name: String,
    pub test_command: String,
    pub default_agent: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            version: 1,
            name: String::from("my-project"),
            test_command: String::from("cargo test"),
            default_agent: String::from("generic-shell"),
        }
    }
}

impl ProjectConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("reading project config from {}", path.display()))?;
        let config: Self = serde_yaml::from_str(&content)
            .with_context(|| "parsing project.yaml")?;
        Ok(config)
    }

    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)
            .with_context(|| format!("writing project config to {}", path.display()))?;
        Ok(())
    }
}
