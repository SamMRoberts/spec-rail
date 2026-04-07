use anyhow::{Context, Result};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::Path,
};

use super::models::LedgerEvent;

pub struct Ledger;

impl Ledger {
    /// Append a single event to the JSONL ledger file.
    pub fn append(path: &Path, event: &LedgerEvent) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("opening ledger at {}", path.display()))?;
        let line = serde_json::to_string(event)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    /// Read all events from the JSONL ledger file.
    pub fn read_all(path: &Path) -> Result<Vec<LedgerEvent>> {
        if !path.exists() {
            return Ok(vec![]);
        }
        let file = std::fs::File::open(path)
            .with_context(|| format!("opening ledger at {}", path.display()))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for (line_num, line) in reader.lines().enumerate() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event: LedgerEvent = serde_json::from_str(trimmed).with_context(|| {
                format!("parsing ledger line {} in {}", line_num + 1, path.display())
            })?;
            events.push(event);
        }
        Ok(events)
    }
}
