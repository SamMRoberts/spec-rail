use anyhow::{Context, Result};

use crate::{
    core::{
        ledger::Ledger,
        models::{LedgerEvent, LedgerEventType, PhaseStatus, VerificationResult, VerificationStatus},
        repository::Repository,
    },
    runtime::process::run_shell,
};

pub fn run(repo: &Repository) -> Result<()> {
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

    let mut phase = repo.load_phase(feature_id, phase_id)?;

    println!("▶ Running verification for phase '{phase_id}'…");
    println!("  Test command: {}", config.test_command);
    println!("{}", "─".repeat(60));

    let output = run_shell(&config.test_command, Some(&repo.root))?;

    let exit_code = output.status.code();
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    // Print output
    if !stdout.is_empty() {
        print!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    println!("{}", "─".repeat(60));

    let verification = VerificationResult {
        status: if success {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        test_command: config.test_command.clone(),
        stdout,
        stderr,
        exit_code,
    };

    // Update phase status
    if success {
        phase.status = PhaseStatus::Verified;
        println!("✓ Verification PASSED.");
        println!("  Run `specrail advance` to move to the next phase.");
    } else {
        phase.status = PhaseStatus::Failed;
        println!("✗ Verification FAILED (exit code: {}).", exit_code.unwrap_or(-1));
        println!("  Fix the failing tests, then run `specrail verify` again.");
    }
    repo.save_phase(&phase)?;

    let event = LedgerEvent::new(LedgerEventType::VerificationRun)
        .with_feature(feature_id)
        .with_phase(phase_id)
        .with_success(success)
        .with_message(format!("exit_code={}", exit_code.unwrap_or(-1)));
    Ledger::append(&repo.ledger_path(), &event)?;

    let phase_event_type = if success {
        LedgerEventType::PhaseVerified
    } else {
        LedgerEventType::PhaseFailed
    };
    let phase_event = LedgerEvent::new(phase_event_type)
        .with_feature(feature_id)
        .with_phase(phase_id)
        .with_success(success);
    Ledger::append(&repo.ledger_path(), &phase_event)?;

    // Suppress unused variable warning — result is persisted via ledger
    let _ = verification;

    Ok(())
}
