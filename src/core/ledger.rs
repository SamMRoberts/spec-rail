use anyhow::Result;

use super::models::LedgerEvent;
use crate::core::repository::Repository;

pub struct Ledger;

impl Ledger {
    /// Append a single event to the SQLite-backed history store.
    pub fn append(repo: &Repository, event: &LedgerEvent) -> Result<()> {
        repo.database()?.append_history_event(event)
    }

    /// Read all events from the SQLite-backed history store.
    pub fn read_all(repo: &Repository) -> Result<Vec<LedgerEvent>> {
        repo.database()?.read_history_events()
    }
}
