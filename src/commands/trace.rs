use anyhow::Result;

use crate::core::{ledger::Ledger, models::LedgerEventType, repository::Repository};

pub fn run(repo: &Repository, limit: Option<usize>) -> Result<()> {
    let events = Ledger::read_all(&repo.ledger_path())?;

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
        "TIMESTAMP", "EVENT", "FEATURE", "PHASE", "DETAILS"
    );
    println!("{}", "─".repeat(120));

    for ev in events_to_show {
        let ts = ev.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let event_name = format_event_type(&ev.event_type);
        let feature = ev.feature_id.as_deref().unwrap_or("—");
        let phase = ev.phase_id.as_deref().unwrap_or("—");
        let details = build_details(ev);
        println!(
            "{:<30} {:<25} {:<20} {:<20} {}",
            ts, event_name, feature, phase, details
        );
    }

    Ok(())
}

fn format_event_type(et: &LedgerEventType) -> &'static str {
    match et {
        LedgerEventType::ProjectInitialized => "project_initialized",
        LedgerEventType::FeatureCreated => "feature_created",
        LedgerEventType::FeatureActivated => "feature_activated",
        LedgerEventType::PhaseCreated => "phase_created",
        LedgerEventType::PhaseActivated => "phase_activated",
        LedgerEventType::TestAdded => "test_added",
        LedgerEventType::ImplementationRun => "implementation_run",
        LedgerEventType::VerificationRun => "verification_run",
        LedgerEventType::PhaseVerified => "phase_verified",
        LedgerEventType::PhaseFailed => "phase_failed",
        LedgerEventType::PhaseAdvanced => "phase_advanced",
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
