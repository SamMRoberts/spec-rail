use anyhow::{bail, Result};

use crate::core::{
    ledger::Ledger,
    models::{ComponentSpec, LedgerEvent, LedgerEventType},
    repository::Repository,
};

pub struct NewArgs {
    pub project_id: String,
    pub id: String,
    pub title: String,
    pub purpose: String,
}

pub struct EditArgs {
    pub id: String,
    pub title: String,
    pub purpose: String,
}

pub(crate) fn create(repo: &Repository, args: NewArgs) -> Result<ComponentSpec> {
    let project = repo.load_project(&args.project_id)?;

    let path = repo.component_path(&args.id);
    if path.exists() {
        bail!("component '{}' already exists at {}", args.id, path.display());
    }

    let component = ComponentSpec {
        id: args.id.clone(),
        solution_id: project.solution_id,
        project_id: project.id,
        title: args.title,
        purpose: args.purpose,
    };
    repo.save_component(&component)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureEdited)
        .with_message(format!("component '{}' created", component.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(component)
}

pub fn new(repo: &Repository, args: NewArgs) -> Result<()> {
    let component = create(repo, args)?;
    println!("✓ Component '{}' created: {}", component.id, component.title);
    println!("  Next: specrail feature new <feature-id> --component {}", component.id);
    Ok(())
}

pub fn list(repo: &Repository, project_id: &str) -> Result<()> {
    repo.load_project(project_id)?;
    let components = repo.list_components(project_id)?;
    if components.is_empty() {
        println!("No components found for project '{project_id}'.");
        return Ok(());
    }

    println!("{:<24} {}", "ID", "TITLE");
    println!("{}", "─".repeat(60));
    for component in &components {
        println!("{:<24} {}", component.id, component.title);
    }

    Ok(())
}

pub fn show(repo: &Repository, id: &str) -> Result<()> {
    let component = repo.load_component(id)?;
    let features = repo.list_features_for_component(&component.id)?;
    println!("Component: {} — {}", component.id, component.title);
    println!("Project:   {}", component.project_id);
    println!("Purpose:   {}", component.purpose);
    println!("Features:  {}", features.len());
    Ok(())
}

pub(crate) fn edit_component(repo: &Repository, args: EditArgs) -> Result<ComponentSpec> {
    let mut component = repo.load_component(&args.id)?;
    component.title = args.title;
    component.purpose = args.purpose;
    repo.save_component(&component)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureEdited)
        .with_message(format!("component '{}' updated", component.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    Ok(component)
}

pub fn edit(repo: &Repository, args: EditArgs) -> Result<()> {
    let component = edit_component(repo, args)?;
    println!("✓ Component '{}' updated: {}", component.id, component.title);
    Ok(())
}

pub fn activate(repo: &Repository, id: &str) -> Result<()> {
    let component = repo.load_component(id)?;

    let mut state = repo.load_state()?;
    state.active_solution = Some(component.solution_id);
    state.active_project = Some(component.project_id);
    state.active_component = Some(component.id.clone());
    state.active_feature = None;
    state.active_outcome = None;
    repo.save_state(&state)?;

    let event = LedgerEvent::new(LedgerEventType::FeatureActivated)
        .with_message(format!("component '{}' activated", id));
    Ledger::append(&repo.ledger_path(), &event)?;

    println!("✓ Component '{id}' is now active.");
    Ok(())
}
