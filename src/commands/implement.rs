use anyhow::{Context, Result};

use crate::{
    agents,
    core::{
        ledger::Ledger,
        models::{AgentTask, LedgerEvent, LedgerEventType, OutcomeStatus},
        repository::Repository,
    },
    policy::outcome_gate,
    prompts::builder,
};

pub fn run(repo: &Repository, agent_override: Option<&str>) -> Result<()> {
    let state = repo.load_state()?;
    let config = repo.load_config()?;

    let feature_id = state
        .active_feature
        .as_deref()
        .context("no active feature — run `specrail feature activate <id>` first")?;

    let outcome_id = state
        .active_outcome
        .as_deref()
        .context("no active outcome — run `specrail outcome activate <feature-id> <outcome-id>` first")?;

    let feature = repo.load_feature(feature_id)?;
    let mut outcome = repo.load_outcome(feature_id, outcome_id)?;
    let manifest = repo.load_manifest()?;

    // Outcome gate checks
    outcome_gate::check_implementation_gates(&outcome, &manifest).with_context(|| {
        format!("outcome gate check failed for outcome '{outcome_id}'")
    })?;

    let agent_name = agent_override.unwrap_or(&config.default_agent);

    let prompt = builder::build_implementation_prompt(&feature, &outcome, &manifest);

    let task = AgentTask {
        feature_id: feature_id.to_string(),
        outcome_id: outcome_id.to_string(),
        agent: agent_name.to_string(),
        prompt,
        allowed_paths: outcome.allowed_paths.clone(),
        forbidden_paths: outcome.forbidden_paths.clone(),
    };

    println!("▶ Running agent '{agent_name}' for outcome '{outcome_id}'…");
    println!("{}", "─".repeat(60));

    let result = agents::run_task(agent_name, &task, Some(&repo.root))?;

    println!("stdout:\n{}", result.stdout);
    if !result.stderr.is_empty() {
        eprintln!("stderr:\n{}", result.stderr);
    }
    println!("{}", "─".repeat(60));

    // Update outcome status to Active if still Pending
    if outcome.status == OutcomeStatus::Pending {
        outcome.status = OutcomeStatus::Active;
        repo.save_outcome(&outcome)?;
    }

    let event = LedgerEvent::new(LedgerEventType::ImplementationRun)
        .with_feature(feature_id)
        .with_outcome(outcome_id)
        .with_agent(agent_name)
        .with_success(result.success);
    Ledger::append(&repo.ledger_path(), &event)?;

    if result.success {
        println!("✓ Agent run complete. Now run `specrail verify` to check the tests.");
    } else {
        println!("✗ Agent exited with non-zero status ({}). Review output above.",
            result.exit_code.unwrap_or(-1));
    }

    Ok(())
}
