use anyhow::{Context, Result};

use crate::{
    core::{
        ledger::Ledger,
        models::{LedgerEvent, LedgerEventType, PhaseStatus},
        repository::Repository,
    },
    policy::phase_gate,
};

pub fn run(repo: &Repository) -> Result<()> {
    let state = repo.load_state()?;

    let feature_id = state
        .active_feature
        .as_deref()
        .context("no active feature")?;

    let phase_id = state
        .active_phase
        .as_deref()
        .context("no active phase")?;

    let phase = repo.load_phase(feature_id, phase_id)?;

    // Gate: current phase must be verified
    phase_gate::check_advance_gates(&phase)?;

    // Find the next phase by order
    let all_phases = repo.list_phases(feature_id)?;
    let next_phase = all_phases
        .iter()
        .find(|p| p.order == phase.order + 1)
        .cloned();

    // Log advancement
    let event = LedgerEvent::new(LedgerEventType::PhaseAdvanced)
        .with_feature(feature_id)
        .with_phase(phase_id)
        .with_message(
            next_phase
                .as_ref()
                .map(|np| format!("advanced to {}", np.id))
                .unwrap_or_else(|| "feature complete".into()),
        );
    Ledger::append(&repo.ledger_path(), &event)?;

    match next_phase {
        Some(mut next) => {
            next.status = PhaseStatus::Active;
            repo.save_phase(&next)?;

            let mut state = repo.load_state()?;
            state.active_phase = Some(next.id.clone());
            repo.save_state(&state)?;

            super::feature::set_current_phase(repo, feature_id, Some(&next.id))?;

            println!("✓ Advanced from '{phase_id}' to '{}'.", next.id);
            println!("  Next: specrail implement");
        }
        None => {
            // No next phase — mark feature complete
            let mut feature = repo.load_feature(feature_id)?;
            feature.status = crate::core::models::FeatureStatus::Complete;
            feature.current_phase = None;
            repo.save_feature(&feature)?;

            let mut state = repo.load_state()?;
            state.active_phase = None;
            repo.save_state(&state)?;

            println!("✓ All phases for feature '{feature_id}' are complete!");
            println!("  Feature marked as complete.");
        }
    }

    Ok(())
}
