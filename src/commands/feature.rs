use anyhow::{bail, Result};

use crate::core::{
        ledger::Ledger,
        models::{FeatureSpec, FeatureStatus, LedgerEvent, LedgerEventType},
        repository::Repository,
    };

// ── feature new ───────────────────────────────────────────────────────────────

pub struct NewArgs {
    pub id: String,
    pub component_id: Option<String>,
    pub title: String,
    pub purpose: String,
    pub outcomes: Vec<String>,
    pub constraints: Vec<String>,
    pub non_goals: Vec<String>,
    pub dependencies: Vec<String>,
}

pub struct EditArgs {
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

    let component_id = resolve_component_id(repo, args.component_id.as_deref())?;
    let component = repo.load_component(&component_id)?;

    let feature = FeatureSpec {
        id: args.id.clone(),
        solution_id: component.solution_id.clone(),
        project_id: component.project_id.clone(),
        component_id: component.id.clone(),
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
    println!(
        "  Hierarchy: {}/{}/{}",
        feature.solution_id, feature.project_id, feature.component_id
    );
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
    println!(
        "{:<20} {:<20} {:<20} {:<24} {:<12} {}",
        "SOLUTION", "PROJECT", "COMPONENT", "ID", "STATUS", "TITLE"
    );
    println!("{}", "─".repeat(128));
    for f in &features {
        let status = format!("{:?}", f.status).to_lowercase();
        println!(
            "{:<20} {:<20} {:<20} {:<24} {:<12} {}",
            f.solution_id, f.project_id, f.component_id, f.id, status, f.title
        );
    }
    Ok(())
}

// ── feature show ──────────────────────────────────────────────────────────────

pub fn show(repo: &Repository, id: &str) -> Result<()> {
    let f = repo.load_feature(id)?;
    println!("Feature: {} — {}", f.id, f.title);
    println!(
        "Hierarchy: {}/{}/{}",
        f.solution_id, f.project_id, f.component_id
    );
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

pub(crate) fn edit_feature(repo: &Repository, args: EditArgs) -> Result<FeatureSpec> {
    let mut feature = repo.load_feature(&args.id)?;

    feature.title = args.title;
    feature.purpose = args.purpose;
    feature.outcomes = args.outcomes;
    feature.constraints = args.constraints;
    feature.non_goals = args.non_goals;
    feature.dependencies = args.dependencies;

    repo.save_feature(&feature)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureEdited)
        .with_feature(&feature.id)
        .with_message(format!("feature '{}' updated", feature.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(feature)
}

pub fn edit(repo: &Repository, args: EditArgs) -> Result<()> {
    let feature = edit_feature(repo, args)?;
    println!("✓ Feature '{}' updated: {}", feature.id, feature.title);
    Ok(())
}

// ── feature activate ──────────────────────────────────────────────────────────

pub(crate) fn activate_feature(repo: &Repository, id: &str) -> Result<()> {
    let mut feature = repo.load_feature(id)?;
    let mut state = repo.load_state()?;

    feature.status = FeatureStatus::Active;
    repo.save_feature(&feature)?;

    state.active_solution = Some(feature.solution_id.clone());
    state.active_project = Some(feature.project_id.clone());
    state.active_component = Some(feature.component_id.clone());
    state.active_feature = Some(id.to_string());
    state.active_outcome = None;
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

fn resolve_component_id(repo: &Repository, requested: Option<&str>) -> Result<String> {
    if let Some(component_id) = requested {
        repo.load_component(component_id)?;
        return Ok(component_id.to_string());
    }

    let state = repo.load_state()?;
    if let Some(component_id) = state.active_component {
        repo.load_component(&component_id)?;
        return Ok(component_id);
    }

    Ok(crate::core::repository::DEFAULT_COMPONENT_ID.to_string())
}
