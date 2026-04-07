use anyhow::{bail, Result};

use crate::core::{
    ledger::Ledger,
    models::{LedgerEvent, LedgerEventType, OutcomeSpec, OutcomeStatus},
    repository::Repository,
};

// ── outcome new ───────────────────────────────────────────────────────────────

pub struct NewArgs {
    pub feature_id: String,
    pub outcome_id: String,
    pub title: String,
    pub goal: String,
    pub order: u32,
    pub prerequisites: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_tests: Vec<String>,
}

pub struct EditArgs {
    pub feature_id: String,
    pub outcome_id: String,
    pub title: String,
    pub goal: String,
    pub order: u32,
    pub prerequisites: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_tests: Vec<String>,
}

pub(crate) fn create(repo: &Repository, args: NewArgs) -> Result<OutcomeSpec> {
    // Ensure the feature exists
    repo.load_feature(&args.feature_id)?;

    let path = repo.outcome_path(&args.feature_id, &args.outcome_id);
    if path.exists() {
        bail!(
            "outcome '{}' already exists for feature '{}'",
            args.outcome_id,
            args.feature_id
        );
    }

    let outcome = OutcomeSpec {
        id: args.outcome_id.clone(),
        feature_id: args.feature_id.clone(),
        title: args.title,
        goal: args.goal,
        order: args.order,
        prerequisites: args.prerequisites,
        allowed_paths: args.allowed_paths,
        forbidden_paths: args.forbidden_paths,
        required_tests: args.required_tests,
        status: OutcomeStatus::Pending,
    };

    repo.save_outcome(&outcome)?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeCreated)
        .with_feature(&args.feature_id)
        .with_outcome(&args.outcome_id);
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(outcome)
}

pub fn new(repo: &Repository, args: NewArgs) -> Result<()> {
    let outcome = create(repo, args)?;
    let path = repo.outcome_path(&outcome.feature_id, &outcome.id);

    println!(
        "✓ Outcome '{}' created for feature '{}'",
        outcome.id, outcome.feature_id
    );
    println!("  Path: {}", path.display());
    Ok(())
}

// ── outcome list ──────────────────────────────────────────────────────────────

pub fn list(repo: &Repository, feature_id: &str) -> Result<()> {
    // Ensure feature exists
    repo.load_feature(feature_id)?;

    let outcomes = repo.list_outcomes(feature_id)?;
    if outcomes.is_empty() {
        println!("No outcomes for feature '{feature_id}'. Add one with `specrail outcome new`.");
        return Ok(());
    }

    println!("{:<5} {:<30} {:<12} {}", "ORD", "ID", "STATUS", "TITLE");
    println!("{}", "─".repeat(75));
    for o in &outcomes {
        let status = format!("{:?}", o.status).to_lowercase();
        println!("{:<5} {:<30} {:<12} {}", o.order, o.id, status, o.title);
    }
    Ok(())
}

// ── outcome show ──────────────────────────────────────────────────────────────

pub fn show(repo: &Repository, feature_id: &str, outcome_id: &str) -> Result<()> {
    let o = repo.load_outcome(feature_id, outcome_id)?;
    println!("Outcome: {} (order {})", o.id, o.order);
    println!("Feature: {}", o.feature_id);
    println!("Title:   {}", o.title);
    println!("Goal:    {}", o.goal);
    println!("Status:  {:?}", o.status);

    if !o.prerequisites.is_empty() {
        println!("\nPrerequisites:");
        for pr in &o.prerequisites {
            println!("  • {pr}");
        }
    }
    if !o.allowed_paths.is_empty() {
        println!("\nAllowed paths:");
        for ap in &o.allowed_paths {
            println!("  • {ap}");
        }
    }
    if !o.forbidden_paths.is_empty() {
        println!("\nForbidden paths:");
        for fp in &o.forbidden_paths {
            println!("  • {fp}");
        }
    }
    if !o.required_tests.is_empty() {
        println!("\nRequired tests:");
        for rt in &o.required_tests {
            println!("  • {rt}");
        }
    }
    Ok(())
}

pub(crate) fn edit_outcome(repo: &Repository, args: EditArgs) -> Result<OutcomeSpec> {
    let mut outcome = repo.load_outcome(&args.feature_id, &args.outcome_id)?;

    outcome.title = args.title;
    outcome.goal = args.goal;
    outcome.order = args.order;
    outcome.prerequisites = args.prerequisites;
    outcome.allowed_paths = args.allowed_paths;
    outcome.forbidden_paths = args.forbidden_paths;
    outcome.required_tests = args.required_tests;

    if matches!(
        outcome.status,
        OutcomeStatus::Verified | OutcomeStatus::Failed | OutcomeStatus::Skipped
    ) {
        outcome.status = OutcomeStatus::Pending;
    }

    repo.save_outcome(&outcome)?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeEdited)
        .with_feature(&outcome.feature_id)
        .with_outcome(&outcome.id);
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(outcome)
}

pub fn edit(repo: &Repository, args: EditArgs) -> Result<()> {
    let outcome = edit_outcome(repo, args)?;
    println!(
        "✓ Outcome '{}' updated for feature '{}'",
        outcome.id, outcome.feature_id
    );
    Ok(())
}

// ── outcome activate ──────────────────────────────────────────────────────────

pub(crate) fn activate_outcome(
    repo: &Repository,
    feature_id: &str,
    outcome_id: &str,
) -> Result<()> {
    let mut outcome = repo.load_outcome(feature_id, outcome_id)?;
    let mut state = repo.load_state()?;

    if outcome.status == OutcomeStatus::Verified {
        bail!("outcome '{outcome_id}' is already verified — use `specrail advance`");
    }

    outcome.status = OutcomeStatus::Active;
    repo.save_outcome(&outcome)?;

    state.active_feature = Some(feature_id.to_string());
    state.active_outcome = Some(outcome_id.to_string());
    repo.save_state(&state)?;

    // Also set the current_outcome on the feature record
    super::feature::set_current_outcome(repo, feature_id, Some(outcome_id))?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeActivated)
        .with_feature(feature_id)
        .with_outcome(outcome_id);
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(())
}

pub fn activate(repo: &Repository, feature_id: &str, outcome_id: &str) -> Result<()> {
    activate_outcome(repo, feature_id, outcome_id)?;

    println!("✓ Outcome '{outcome_id}' is now active for feature '{feature_id}'.");
    Ok(())
}
