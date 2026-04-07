use anyhow::{bail, Result};

use crate::core::{
        ledger::Ledger,
        models::{FeatureSpec, FeatureStatus, LedgerEvent, LedgerEventType},
        repository::Repository,
    };

// ── feature new ───────────────────────────────────────────────────────────────

pub struct NewArgs {
    pub id: String,
    pub title: String,
    pub purpose: String,
    pub outcomes: Vec<String>,
    pub constraints: Vec<String>,
    pub non_goals: Vec<String>,
    pub dependencies: Vec<String>,
}

pub(crate) fn create(repo: &Repository, args: NewArgs) -> Result<FeatureSpec> {
    let path = repo.feature_path(&args.id);
    if path.exists() {
        bail!("feature '{}' already exists at {}", args.id, path.display());
    }

    let feature = FeatureSpec {
        id: args.id.clone(),
        title: args.title,
        purpose: args.purpose,
        outcomes: args.outcomes,
        constraints: args.constraints,
        non_goals: args.non_goals,
        dependencies: args.dependencies,
        status: FeatureStatus::Draft,
        current_outcome: None,
    };

    repo.save_feature(&feature)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureCreated)
        .with_feature(&args.id)
        .with_message(format!("feature '{}' created", args.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(feature)
}

pub fn new(repo: &Repository, args: NewArgs) -> Result<()> {
    let feature = create(repo, args)?;
    let path = repo.feature_path(&feature.id);

    println!("✓ Feature '{}' created: {}", feature.id, feature.title);
    println!("  Path: {}", path.display());
    println!("  Next: specrail outcome new {} <outcome-id>", feature.id);
    Ok(())
}

// ── feature list ──────────────────────────────────────────────────────────────

pub fn list(repo: &Repository) -> Result<()> {
    let features = repo.list_features()?;
    if features.is_empty() {
        println!("No features found. Run `specrail feature new <id>` to create one.");
        return Ok(());
    }
    println!("{:<30} {:<12} {}", "ID", "STATUS", "TITLE");
    println!("{}", "─".repeat(70));
    for f in &features {
        let status = format!("{:?}", f.status).to_lowercase();
        println!("{:<30} {:<12} {}", f.id, status, f.title);
    }
    Ok(())
}

// ── feature show ──────────────────────────────────────────────────────────────

pub fn show(repo: &Repository, id: &str) -> Result<()> {
    let f = repo.load_feature(id)?;
    println!("Feature: {} — {}", f.id, f.title);
    println!("Status:  {:?}", f.status);
    println!("Purpose: {}", f.purpose);

    if !f.outcomes.is_empty() {
        println!("\nOutcomes:");
        for o in &f.outcomes {
            println!("  • {o}");
        }
    }
    if !f.constraints.is_empty() {
        println!("\nConstraints:");
        for c in &f.constraints {
            println!("  • {c}");
        }
    }
    if !f.non_goals.is_empty() {
        println!("\nNon-goals:");
        for ng in &f.non_goals {
            println!("  • {ng}");
        }
    }
    if let Some(outcome) = &f.current_outcome {
        println!("\nCurrent outcome: {outcome}");
    }
    Ok(())
}

// ── feature activate ──────────────────────────────────────────────────────────

pub(crate) fn activate_feature(repo: &Repository, id: &str) -> Result<()> {
    let mut feature = repo.load_feature(id)?;
    let mut state = repo.load_state()?;

    feature.status = FeatureStatus::Active;
    repo.save_feature(&feature)?;

    state.active_feature = Some(id.to_string());
    repo.save_state(&state)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureActivated).with_feature(id);
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(())
}

pub fn activate(repo: &Repository, id: &str) -> Result<()> {
    activate_feature(repo, id)?;

    println!("✓ Feature '{id}' is now active.");
    Ok(())
}

// ── feature set-outcome ───────────────────────────────────────────────────────

/// Update the `current_outcome` field on a feature (called by `advance`).
pub fn set_current_outcome(
    repo: &Repository,
    feature_id: &str,
    outcome_id: Option<&str>,
) -> Result<()> {
    let mut feature = repo.load_feature(feature_id)?;
    feature.current_outcome = outcome_id.map(str::to_string);
    repo.save_feature(&feature)
}
