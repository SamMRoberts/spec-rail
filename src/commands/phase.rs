use anyhow::{bail, Result};

use crate::core::{
    ledger::Ledger,
    models::{LedgerEvent, LedgerEventType, PhaseSpec, PhaseStatus},
    repository::Repository,
};

// ── phase new ─────────────────────────────────────────────────────────────────

pub struct NewArgs {
    pub feature_id: String,
    pub phase_id: String,
    pub title: String,
    pub goal: String,
    pub order: u32,
    pub prerequisites: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_tests: Vec<String>,
}

pub(crate) fn create(repo: &Repository, args: NewArgs) -> Result<PhaseSpec> {
    // Ensure the feature exists
    repo.load_feature(&args.feature_id)?;

    let path = repo.phase_path(&args.feature_id, &args.phase_id);
    if path.exists() {
        bail!(
            "phase '{}' already exists for feature '{}'",
            args.phase_id,
            args.feature_id
        );
    }

    let phase = PhaseSpec {
        id: args.phase_id.clone(),
        feature_id: args.feature_id.clone(),
        title: args.title,
        goal: args.goal,
        order: args.order,
        prerequisites: args.prerequisites,
        allowed_paths: args.allowed_paths,
        forbidden_paths: args.forbidden_paths,
        required_tests: args.required_tests,
        status: PhaseStatus::Pending,
    };

    repo.save_phase(&phase)?;

    let event = LedgerEvent::new(LedgerEventType::PhaseCreated)
        .with_feature(&args.feature_id)
        .with_phase(&args.phase_id);
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(phase)
}

pub fn new(repo: &Repository, args: NewArgs) -> Result<()> {
    let phase = create(repo, args)?;
    let path = repo.phase_path(&phase.feature_id, &phase.id);

    println!(
        "✓ Phase '{}' created for feature '{}'",
        phase.id, phase.feature_id
    );
    println!("  Path: {}", path.display());
    Ok(())
}

// ── phase list ────────────────────────────────────────────────────────────────

pub fn list(repo: &Repository, feature_id: &str) -> Result<()> {
    // Ensure feature exists
    repo.load_feature(feature_id)?;

    let phases = repo.list_phases(feature_id)?;
    if phases.is_empty() {
        println!("No phases for feature '{feature_id}'. Add one with `specrail phase new`.");
        return Ok(());
    }

    println!("{:<5} {:<30} {:<12} {}", "ORD", "ID", "STATUS", "TITLE");
    println!("{}", "─".repeat(75));
    for p in &phases {
        let status = format!("{:?}", p.status).to_lowercase();
        println!("{:<5} {:<30} {:<12} {}", p.order, p.id, status, p.title);
    }
    Ok(())
}

// ── phase show ────────────────────────────────────────────────────────────────

pub fn show(repo: &Repository, feature_id: &str, phase_id: &str) -> Result<()> {
    let p = repo.load_phase(feature_id, phase_id)?;
    println!("Phase:   {} (order {})", p.id, p.order);
    println!("Feature: {}", p.feature_id);
    println!("Title:   {}", p.title);
    println!("Goal:    {}", p.goal);
    println!("Status:  {:?}", p.status);

    if !p.prerequisites.is_empty() {
        println!("\nPrerequisites:");
        for pr in &p.prerequisites {
            println!("  • {pr}");
        }
    }
    if !p.allowed_paths.is_empty() {
        println!("\nAllowed paths:");
        for ap in &p.allowed_paths {
            println!("  • {ap}");
        }
    }
    if !p.forbidden_paths.is_empty() {
        println!("\nForbidden paths:");
        for fp in &p.forbidden_paths {
            println!("  • {fp}");
        }
    }
    if !p.required_tests.is_empty() {
        println!("\nRequired tests:");
        for rt in &p.required_tests {
            println!("  • {rt}");
        }
    }
    Ok(())
}

// ── phase activate ────────────────────────────────────────────────────────────

pub(crate) fn activate_phase(repo: &Repository, feature_id: &str, phase_id: &str) -> Result<()> {
    let mut phase = repo.load_phase(feature_id, phase_id)?;
    let mut state = repo.load_state()?;

    if phase.status == PhaseStatus::Verified {
        bail!("phase '{phase_id}' is already verified — use `specrail advance`");
    }

    phase.status = PhaseStatus::Active;
    repo.save_phase(&phase)?;

    state.active_feature = Some(feature_id.to_string());
    state.active_phase = Some(phase_id.to_string());
    repo.save_state(&state)?;

    // Also set the current_phase on the feature record
    super::feature::set_current_phase(repo, feature_id, Some(phase_id))?;

    let event = LedgerEvent::new(LedgerEventType::PhaseActivated)
        .with_feature(feature_id)
        .with_phase(phase_id);
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(())
}

pub fn activate(repo: &Repository, feature_id: &str, phase_id: &str) -> Result<()> {
    activate_phase(repo, feature_id, phase_id)?;

    println!("✓ Phase '{phase_id}' is now active for feature '{feature_id}'.");
    Ok(())
}
