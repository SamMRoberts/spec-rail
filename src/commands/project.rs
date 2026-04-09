use anyhow::{bail, Result};

use crate::core::{
    config::ProjectSettings,
    ledger::Ledger,
    models::{LedgerEvent, LedgerEventType, ProjectSpec},
    repository::Repository,
};

pub struct NewArgs {
    pub solution_id: String,
    pub id: String,
    pub title: String,
    pub purpose: String,
    pub test_command: Option<String>,
}

pub struct EditArgs {
    pub id: String,
    pub title: String,
    pub purpose: String,
}

pub(crate) fn create(repo: &Repository, args: NewArgs) -> Result<ProjectSpec> {
    repo.load_solution(&args.solution_id)?;

    if repo.project_exists(&args.id)? {
        bail!("project '{}' already exists", args.id);
    }

    let project = ProjectSpec {
        id: args.id.clone(),
        solution_id: args.solution_id,
        title: args.title,
        purpose: args.purpose,
    };
    repo.save_project(&project)?;

    // Create the per-project settings file so each specrail project has its
    // own project.yaml with the test_command for its software project.
    let settings = ProjectSettings {
        test_command: args
            .test_command
            .unwrap_or_else(|| ProjectSettings::for_project(&repo.root).test_command),
        default_agent: None,
    };
    repo.save_project_settings(&project.id, &settings)?;

    let event = LedgerEvent::new(LedgerEventType::ProjectCreated)
        .with_message(format!("project '{}' created", project.id));
    Ledger::append(repo, &event)?;

    Ok(project)
}

pub fn new(repo: &Repository, args: NewArgs) -> Result<()> {
    let project = create(repo, args)?;
    println!("✓ Project '{}' created: {}", project.id, project.title);
    println!("  Stored in .specrail/specrail.db");
    println!(
        "  Settings:  .specrail/projects/{}.yaml (edit to set test_command)",
        project.id
    );
    println!("  Next: specrail component new {} <component-id>", project.id);
    Ok(())
}

pub fn list(repo: &Repository, solution_id: &str) -> Result<()> {
    repo.load_solution(solution_id)?;
    let projects = repo.list_projects(solution_id)?;
    if projects.is_empty() {
        println!("No projects found for solution '{solution_id}'.");
        return Ok(());
    }

    println!("{:<24} {}", "ID", "TITLE");
    println!("{}", "─".repeat(60));
    for project in &projects {
        println!("{:<24} {}", project.id, project.title);
    }

    Ok(())
}

pub fn show(repo: &Repository, id: &str) -> Result<()> {
    let project = repo.load_project(id)?;
    let components = repo.list_components(&project.id)?;
    println!("Project:  {} — {}", project.id, project.title);
    println!("Solution: {}", project.solution_id);
    println!("Purpose:  {}", project.purpose);
    if let Ok(settings) = repo.load_project_settings(id) {
        println!("Test command: {}", settings.test_command);
        if let Some(agent) = &settings.default_agent {
            println!("Default agent: {agent}");
        }
    }
    println!("Components: {}", components.len());
    Ok(())
}

pub(crate) fn edit_project(repo: &Repository, args: EditArgs) -> Result<ProjectSpec> {
    let mut project = repo.load_project(&args.id)?;
    project.title = args.title;
    project.purpose = args.purpose;
    repo.save_project(&project)?;

    let event = LedgerEvent::new(LedgerEventType::ProjectEdited)
        .with_message(format!("project '{}' updated", project.id));
    Ledger::append(repo, &event)?;

    Ok(project)
}

pub fn edit(repo: &Repository, args: EditArgs) -> Result<()> {
    let project = edit_project(repo, args)?;
    println!("✓ Project '{}' updated: {}", project.id, project.title);
    Ok(())
}

pub fn activate(repo: &Repository, id: &str) -> Result<()> {
    let project = repo.load_project(id)?;

    let mut state = repo.load_state()?;
    state.active_solution = Some(project.solution_id);
    state.active_project = Some(project.id.clone());
    state.active_component = None;
    state.active_feature = None;
    state.active_outcome = None;
    repo.save_state(&state)?;

    let event = LedgerEvent::new(LedgerEventType::ProjectActivated)
        .with_message(format!("project '{}' activated", id));
    Ledger::append(repo, &event)?;

    println!("✓ Project '{id}' is now active.");
    Ok(())
}
