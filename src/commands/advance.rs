use anyhow::{Context, Result};

use crate::{
    core::{
        ledger::Ledger,
        models::{LedgerEvent, LedgerEventType, OutcomeStatus},
        repository::Repository,
    },
    policy::outcome_gate,
};

pub fn run(repo: &Repository) -> Result<()> {
    let state = repo.load_state()?;

    let feature_id = state
        .active_feature
        .as_deref()
        .context("no active feature")?;

    let outcome_id = state
        .active_outcome
        .as_deref()
        .context("no active outcome")?;

    let outcome = repo.load_outcome(feature_id, outcome_id)?;

    // Gate: current outcome must be verified
    outcome_gate::check_advance_gates(&outcome)?;

    // Find the next outcome by order
    let all_outcomes = repo.list_outcomes(feature_id)?;
    let next_outcome = all_outcomes
        .iter()
        .find(|o| o.order == outcome.order + 1)
        .cloned();

    // Log advancement
    let event = LedgerEvent::new(LedgerEventType::OutcomeAdvanced)
        .with_feature(feature_id)
        .with_outcome(outcome_id)
        .with_message(
            next_outcome
                .as_ref()
                .map(|no| format!("advanced to {}", no.id))
                .unwrap_or_else(|| "feature complete".into()),
        );
    Ledger::append(&repo.ledger_path(), &event)?;

    match next_outcome {
        Some(mut next) => {
            next.status = OutcomeStatus::Active;
            repo.save_outcome(&next)?;

            let mut state = repo.load_state()?;
            state.active_outcome = Some(next.id.clone());
            repo.save_state(&state)?;

            super::feature::set_current_outcome(repo, feature_id, Some(&next.id))?;

            println!("✓ Advanced from '{outcome_id}' to '{}'.", next.id);
            println!("  Next: specrail implement");
        }
        None => {
            // No next outcome — mark feature complete
            let mut feature = repo.load_feature(feature_id)?;
            feature.status = crate::core::models::FeatureStatus::Complete;
            feature.current_outcome = None;
            repo.save_feature(&feature)?;

            let mut state = repo.load_state()?;
            state.active_outcome = None;
            repo.save_state(&state)?;

            println!("✓ All outcomes for feature '{feature_id}' are complete!");
            println!("  Feature marked as complete.");
        }
    }

    Ok(())
}
