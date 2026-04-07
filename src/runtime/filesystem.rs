use anyhow::{Context, Result};
use std::path::Path;

/// Ensure a directory exists, creating it (and all parents) if needed.
pub fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("creating directory {}", path.display()))
}

/// Write `content` to `path`, creating parent directories as needed.
/// Does **not** overwrite an existing file.
pub fn write_if_missing(path: &Path, content: &str) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    std::fs::write(path, content)
        .with_context(|| format!("writing file {}", path.display()))?;
    Ok(true)
}

/// Write `content` to `path`, always overwriting.
#[allow(dead_code)]
pub fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    std::fs::write(path, content)
        .with_context(|| format!("writing file {}", path.display()))
}
