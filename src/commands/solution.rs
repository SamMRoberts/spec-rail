use anyhow::{bail, Result};

use crate::core::{
    ledger::Ledger,
    models::{LedgerEvent, LedgerEventType, SolutionSpec},
    repository::Repository,
};

pub struct NewArgs {
    pub id: String,
    pub title: String,
    pub purpose: String,
}

pub struct EditArgs {
    pub id: String,
    pub title: String,
    pub purpose: String,
}

pub(crate) fn create(repo: &Repository, args: NewArgs) -> Result<SolutionSpec> {
    let path = repo.solution_path(&args.id);
    if path.exists() {
        bail!("solution '{}' already exists at {}", args.id, path.display());
    }

    let solution = SolutionSpec {
        id: args.id.clone(),
        title: args.title,
        purpose: args.purpose,
    };
    repo.save_solution(&solution)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureEdited)
        .with_message(format!("solution '{}' created", solution.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(solution)
}

pub fn new(repo: &Repository, args: NewArgs) -> Result<()> {
    let solution = create(repo, args)?;
    println!("✓ Solution '{}' created: {}", solution.id, solution.title);
    println!("  Next: specrail project new {} <project-id>", solution.id);
    Ok(())
}

pub fn list(repo: &Repository) -> Result<()> {
    let solutions = repo.list_solutions()?;
    if solutions.is_empty() {
        println!("No solutions found. Run `specrail solution new <id>` to create one.");
        return Ok(());
    }

    println!("{:<24} {}", "ID", "TITLE");
    println!("{}", "─".repeat(60));
    for solution in &solutions {
        println!("{:<24} {}", solution.id, solution.title);
    }

    Ok(())
}

pub fn show(repo: &Repository, id: &str) -> Result<()> {
    let solution = repo.load_solution(id)?;
    let projects = repo.list_projects(id)?;
    println!("Solution: {} — {}", solution.id, solution.title);
    println!("Purpose:  {}", solution.purpose);
    println!("Projects: {}", projects.len());
    Ok(())
}

pub(crate) fn edit_solution(repo: &Repository, args: EditArgs) -> Result<SolutionSpec> {
    let mut solution = repo.load_solution(&args.id)?;
    solution.title = args.title;
    solution.purpose = args.purpose;
    repo.save_solution(&solution)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureEdited)
        .with_message(format!("solution '{}' updated", solution.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(solution)
}

pub fn edit(repo: &Repository, args: EditArgs) -> Result<()> {
    let solution = edit_solution(repo, args)?;
    println!("✓ Solution '{}' updated: {}", solution.id, solution.title);
    Ok(())
}

pub fn activate(repo: &Repository, id: &str) -> Result<()> {
    repo.load_solution(id)?;

    let mut state = repo.load_state()?;
    state.active_solution = Some(id.to_string());
    state.active_project = None;
    state.active_component = None;
    state.active_feature = None;
    state.active_outcome = None;
    repo.save_state(&state)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureActivated)
        .with_message(format!("solution '{}' activated", id));
    Ledger::append(&repo.ledger_path(), &event)?;

    println!("✓ Solution '{id}' is now active.");
    Ok(())
}
