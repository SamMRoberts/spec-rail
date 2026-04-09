use anyhow::{Context, Result};

use crate::{
    core::{
        ledger::Ledger,
        models::{LedgerEvent, LedgerEventType, OutcomeStatus, VerificationResult, VerificationStatus},
        repository::Repository,
    },
    runtime::process::run_shell,
};

pub fn run(repo: &Repository) -> Result<()> {
    let state = repo.load_state()?;
    let config = repo.effective_config()?;

    let feature_id = state
        .active_feature
        .as_deref()
        .context("no active feature — run `specrail feature activate <id>` first")?;

    let outcome_id = state
        .active_outcome
        .as_deref()
        .context("no active outcome — run `specrail outcome activate <feature-id> <outcome-id>` first")?;

    let mut outcome = repo.load_outcome(feature_id, outcome_id)?;
    let manifest = repo.load_manifest()?;

    crate::policy::outcome_gate::check_verify_gates(repo, &outcome, &manifest).with_context(
        || format!("outcome gate check failed for outcome '{outcome_id}'"),
    )?;

    println!("▶ Running verification for outcome '{outcome_id}'…");
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

    // Update outcome status
    if success {
        outcome.status = OutcomeStatus::Verified;
        println!("✓ Verification PASSED.");
        println!("  Run `specrail advance` to move to the next outcome.");
    } else {
        outcome.status = OutcomeStatus::Failed;
        println!("✗ Verification FAILED (exit code: {}).", exit_code.unwrap_or(-1));
        println!("  Fix the failing tests, then run `specrail verify` again.");
    }
    repo.save_outcome(&outcome)?;

    let event = LedgerEvent::new(LedgerEventType::VerificationRun)
        .with_feature(feature_id)
        .with_outcome(outcome_id)
        .with_success(success)
        .with_message(format!("exit_code={}", exit_code.unwrap_or(-1)));
    Ledger::append(repo, &event)?;

    let outcome_event_type = if success {
        LedgerEventType::OutcomeVerified
    } else {
        LedgerEventType::OutcomeFailed
    };
    let outcome_event = LedgerEvent::new(outcome_event_type)
        .with_feature(feature_id)
        .with_outcome(outcome_id)
        .with_success(success);
    Ledger::append(repo, &outcome_event)?;

    // Suppress unused variable warning — result is persisted via ledger
    let _ = verification;

    Ok(())
}
