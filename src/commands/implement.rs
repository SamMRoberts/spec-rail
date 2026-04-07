use anyhow::{Context, Result};

use crate::{
    agents,
    core::{
        ledger::Ledger,
        models::{AgentTask, LedgerEvent, LedgerEventType, PhaseStatus},
        repository::Repository,
    },
    policy::phase_gate,
    prompts::builder,
};

pub fn run(repo: &Repository, agent_override: Option<&str>) -> Result<()> {
    let state = repo.load_state()?;
    let config = repo.load_config()?;

    let feature_id = state
        .active_feature
        .as_deref()
        .context("no active feature — run `specrail feature activate <id>` first")?;

    let phase_id = state
        .active_phase
        .as_deref()
        .context("no active phase — run `specrail phase activate <feature-id> <phase-id>` first")?;

    let feature = repo.load_feature(feature_id)?;
    let mut phase = repo.load_phase(feature_id, phase_id)?;
    let manifest = repo.load_manifest()?;

    // Phase gate checks
    phase_gate::check_implementation_gates(&phase, &manifest).with_context(|| {
        format!("phase gate check failed for phase '{phase_id}'")
    })?;

    let agent_name = agent_override.unwrap_or(&config.default_agent);

    let prompt = builder::build_implementation_prompt(&feature, &phase, &manifest);

    let task = AgentTask {
        feature_id: feature_id.to_string(),
        phase_id: phase_id.to_string(),
        agent: agent_name.to_string(),
        prompt,
        allowed_paths: phase.allowed_paths.clone(),
        forbidden_paths: phase.forbidden_paths.clone(),
    };

    println!("▶ Running agent '{agent_name}' for phase '{phase_id}'…");
    println!("{}", "─".repeat(60));

    let result = agents::run_task(agent_name, &task, Some(&repo.root))?;

    println!("stdout:\n{}", result.stdout);
    if !result.stderr.is_empty() {
        eprintln!("stderr:\n{}", result.stderr);
    }
    println!("{}", "─".repeat(60));

    // Update phase status to Active if still Pending
    if phase.status == PhaseStatus::Pending {
        phase.status = PhaseStatus::Active;
        repo.save_phase(&phase)?;
    }

    let event = LedgerEvent::new(LedgerEventType::ImplementationRun)
        .with_feature(feature_id)
        .with_phase(phase_id)
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
