use anyhow::Result;

use crate::{
    core::{
        config::ProjectConfig,
        ledger::Ledger,
        models::{LedgerEvent, LedgerEventType},
        repository::Repository,
    },
    runtime::filesystem::{ensure_dir, write_if_missing},
};

pub struct InitOutcome {
    pub should_start_wizard: bool,
}

pub fn run(repo: &Repository, no_wizard: bool) -> Result<InitOutcome> {
    let specrail = repo.specrail_dir();
    let already_initialized = specrail.is_dir();

    if already_initialized {
        println!("✓ .specrail/ already exists — refreshing missing files");
    } else {
        println!("Initializing specrail project…");
    }

    // Create directory structure
    for dir in [
        &specrail,
        &repo.state_dir(),
        &repo.agents_dir(),
    ] {
        ensure_dir(dir)?;
    }

    let db_exists = repo.db_path().exists();
    repo.initialize_database()?;
    if !db_exists {
        println!("  created  .specrail/specrail.db");
    }

    // project.yaml
    let config_path = repo.project_config_path();
    if write_if_missing(
        &config_path,
        &serde_yaml::to_string(&ProjectConfig::default())?,
    )? {
        println!("  created  .specrail/project.yaml");
    }

    // state/ledger.jsonl
    let ledger_path = repo.ledger_path();
    if write_if_missing(&ledger_path, "")? {
        println!("  created  .specrail/state/ledger.jsonl");
    }

    if no_wizard {
        repo.ensure_hierarchy()?;
    }

    // Record the init event
    let event = LedgerEvent::new(LedgerEventType::ProjectInitialized)
        .with_message("specrail project initialized");
    Ledger::append(&ledger_path, &event)?;

    let should_start_wizard = repo.list_features()?.is_empty();

    println!("\n✓ specrail project ready.");
    if should_start_wizard && !no_wizard {
        println!("  Starting setup walkthrough (solution → project → component → feature)...");
    } else {
        println!("  Next steps:");
        println!("    specrail solution new <id> --title <title> --purpose <purpose>");
        println!("    specrail project new <solution-id> <id> --title <title> --purpose <purpose>");
        println!("    specrail component new <project-id> <id> --title <title> --purpose <purpose>");
        println!("    specrail feature new <id> --title <title> --purpose <purpose>");
    }

    Ok(InitOutcome {
        should_start_wizard,
    })
}
