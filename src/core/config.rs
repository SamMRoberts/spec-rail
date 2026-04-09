use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

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
    pub fn for_workspace(root: &Path) -> Self {
        let mut config = Self::default();
        config.test_command = detect_test_command(root);
        config
    }

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

fn detect_test_command(root: &Path) -> String {
    if root.join("Cargo.toml").is_file() {
        return String::from("cargo test");
    }

    if let Some(csproj) = find_preferred_file(root, "csproj", &is_dotnet_test_project) {
        return format!("dotnet test --project {}", shell_quote(&relative_display(root, &csproj)));
    }

    if let Some(package_json) = find_top_level_file(root, "package.json") {
        if root.join("pnpm-lock.yaml").is_file() {
            return format!("pnpm test --dir {}", shell_quote(&relative_display(root, &package_json.parent().unwrap_or(root).to_path_buf())));
        }
        if root.join("yarn.lock").is_file() {
            return format!("yarn --cwd {} test", shell_quote(&relative_display(root, &package_json.parent().unwrap_or(root).to_path_buf())));
        }
        return format!("npm test --prefix {}", shell_quote(&relative_display(root, &package_json.parent().unwrap_or(root).to_path_buf())));
    }

    if root.join("go.mod").is_file() {
        return String::from("go test ./...");
    }

    if root.join("pyproject.toml").is_file()
        || root.join("requirements.txt").is_file()
        || root.join("setup.py").is_file()
    {
        return String::from("pytest");
    }

    if root.join("pom.xml").is_file() {
        return String::from("mvn test");
    }

    if root.join("build.gradle").is_file() || root.join("build.gradle.kts").is_file() {
        if root.join("gradlew").is_file() {
            return String::from("./gradlew test");
        }
        return String::from("gradle test");
    }

    String::from("cargo test")
}

fn find_top_level_file(root: &Path, file_name: &str) -> Option<std::path::PathBuf> {
    let path = root.join(file_name);
    path.is_file().then_some(path)
}

fn find_preferred_file<F>(root: &Path, extension: &str, preferred: &F) -> Option<std::path::PathBuf>
where
    F: Fn(&Path) -> bool,
{
    let mut matches = WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some(extension))
        .collect::<Vec<_>>();

    matches.sort();
    matches
        .iter()
        .find(|path| preferred(path))
        .cloned()
        .or_else(|| matches.into_iter().next())
}

fn is_dotnet_test_project(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    file_name.contains("test")
}

fn relative_display(root: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    if relative.is_empty() {
        String::from(".")
    } else {
        relative
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return String::from("''");
    }

    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-'))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}
