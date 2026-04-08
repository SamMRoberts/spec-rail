use anyhow::Result;

use crate::{
    core::{
        config::ProjectConfig,
        ledger::Ledger,
        models::{LedgerEvent, LedgerEventType, ProjectState, TestManifest},
        repository::Repository,
    },
    runtime::filesystem::{ensure_dir, write_if_missing},
};

pub struct InitOutcome {
    pub should_start_wizard: bool,
}

pub fn run(repo: &Repository) -> Result<InitOutcome> {
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
        &repo.solutions_dir(),
        &repo.projects_dir(),
        &repo.components_dir(),
        &repo.features_dir(),
        &repo.outcomes_dir(),
        &repo.tests_dir(),
        &repo.state_dir(),
        &repo.agents_dir(),
    ] {
        ensure_dir(dir)?;
    }

    // project.yaml
    let config_path = repo.project_config_path();
    if write_if_missing(
        &config_path,
        &serde_yaml::to_string(&ProjectConfig::default())?,
    )? {
        println!("  created  .specrail/project.yaml");
    }

    // tests/manifest.yaml
    let manifest_path = repo.manifest_path();
    if write_if_missing(
        &manifest_path,
        &serde_yaml::to_string(&TestManifest::default())?,
    )? {
        println!("  created  .specrail/tests/manifest.yaml");
    }

    // state/current.yaml
    let state_path = repo.state_path();
    if write_if_missing(
        &state_path,
        &serde_yaml::to_string(&ProjectState::default())?,
    )? {
        println!("  created  .specrail/state/current.yaml");
    }

    // state/ledger.jsonl
    let ledger_path = repo.ledger_path();
    if write_if_missing(&ledger_path, "")? {
        println!("  created  .specrail/state/ledger.jsonl");
    }

    repo.ensure_hierarchy()?;

    // Record the init event
    let event = LedgerEvent::new(LedgerEventType::ProjectInitialized)
        .with_message("specrail project initialized");
    Ledger::append(&ledger_path, &event)?;

    let should_start_wizard = repo.list_features()?.is_empty();

    println!("\n✓ specrail project ready.");
    if should_start_wizard {
        println!("  Starting setup walkthrough...");
    } else {
        println!("  Next steps:");
        println!("    specrail feature new <id> --title <title> --purpose <purpose>");
    }

    Ok(InitOutcome {
        should_start_wizard,
    })
}
