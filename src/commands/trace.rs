use anyhow::Result;

use crate::core::{ledger::Ledger, models::LedgerEventType, repository::Repository};

pub fn run(repo: &Repository, limit: Option<usize>) -> Result<()> {
    let events = Ledger::read_all(repo)?;

    if events.is_empty() {
        println!("No ledger events recorded yet.");
        return Ok(());
    }

    let events_to_show: Vec<_> = match limit {
        Some(n) => events.iter().rev().take(n).collect(),
        None => events.iter().collect(),
    };
    // Reverse back to chronological if we took from the end
    let events_to_show: Vec<_> = if limit.is_some() {
        events_to_show.into_iter().rev().collect()
    } else {
        events_to_show
    };

    println!(
        "{:<30} {:<25} {:<20} {:<20} {}",
        "TIMESTAMP", "EVENT", "FEATURE", "OUTCOME", "DETAILS"
    );
    println!("{}", "─".repeat(120));

    for ev in events_to_show {
        let ts = ev.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let event_name = format_event_type(&ev.event_type);
        let feature = ev.feature_id.as_deref().unwrap_or("—");
        let outcome = ev.outcome_id.as_deref().unwrap_or("—");
        let details = build_details(ev);
        println!(
            "{:<30} {:<25} {:<20} {:<20} {}",
            ts, event_name, feature, outcome, details
        );
    }

    Ok(())
}

fn format_event_type(et: &LedgerEventType) -> &'static str {
    match et {
        LedgerEventType::ProjectInitialized => "project_initialized",
        LedgerEventType::SolutionCreated => "solution_created",
        LedgerEventType::SolutionEdited => "solution_edited",
        LedgerEventType::SolutionActivated => "solution_activated",
        LedgerEventType::ProjectCreated => "project_created",
        LedgerEventType::ProjectEdited => "project_edited",
        LedgerEventType::ProjectActivated => "project_activated",
        LedgerEventType::ComponentCreated => "component_created",
        LedgerEventType::ComponentEdited => "component_edited",
        LedgerEventType::ComponentActivated => "component_activated",
        LedgerEventType::FeatureCreated => "feature_created",
        LedgerEventType::FeatureEdited => "feature_edited",
        LedgerEventType::FeatureActivated => "feature_activated",
        LedgerEventType::OutcomeCreated => "outcome_created",
        LedgerEventType::OutcomeEdited => "outcome_edited",
        LedgerEventType::OutcomeActivated => "outcome_activated",
        LedgerEventType::TestAdded => "test_added",
        LedgerEventType::TestGenerationRun => "test_generation_run",
        LedgerEventType::ImplementationRun => "implementation_run",
        LedgerEventType::VerificationRun => "verification_run",
        LedgerEventType::OutcomeVerified => "outcome_verified",
        LedgerEventType::OutcomeFailed => "outcome_failed",
        LedgerEventType::OutcomeAdvanced => "outcome_advanced",
    }
}

fn build_details(ev: &crate::core::models::LedgerEvent) -> String {
    let mut parts = Vec::new();
    if let Some(agent) = &ev.agent {
        parts.push(format!("agent={agent}"));
    }
    if let Some(success) = ev.success {
        parts.push(format!("success={success}"));
    }
    if let Some(msg) = &ev.message {
        parts.push(msg.clone());
    }
    if parts.is_empty() {
        "—".into()
    } else {
        parts.join(" ")
    }
}
