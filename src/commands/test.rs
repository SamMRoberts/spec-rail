use anyhow::{bail, Result};

use crate::core::{
    ledger::Ledger,
    models::{LedgerEvent, LedgerEventType, TestSpec},
    repository::Repository,
};

// ── test add ──────────────────────────────────────────────────────────────────

pub struct AddArgs {
    pub id: String,
    pub feature_id: String,
    pub phase_id: String,
    pub path: String,
    pub kind: crate::core::models::TestKind,
    pub purpose_refs: Vec<String>,
}

pub fn add(repo: &Repository, args: AddArgs) -> Result<()> {
    // Verify feature and phase exist
    repo.load_feature(&args.feature_id)?;
    repo.load_phase(&args.feature_id, &args.phase_id)?;

    let mut manifest = repo.load_manifest()?;

    if manifest.tests.iter().any(|t| t.id == args.id) {
        bail!("test '{}' already exists in the manifest", args.id);
    }

    let test = TestSpec {
        id: args.id.clone(),
        feature_id: args.feature_id.clone(),
        phase_id: args.phase_id.clone(),
        path: args.path.clone(),
        purpose_refs: args.purpose_refs,
        kind: args.kind,
        status: crate::core::models::TestStatus::Planned,
    };

    manifest.tests.push(test);
    repo.save_manifest(&manifest)?;

    let event = LedgerEvent::new(LedgerEventType::TestAdded)
        .with_feature(&args.feature_id)
        .with_phase(&args.phase_id)
        .with_message(format!("test '{}' added", args.id));
    Ledger::append(&repo.ledger_path(), &event)?;

    println!("✓ Test '{}' added to manifest.", args.id);
    println!("  Path:    {}", args.path);
    println!(
        "  Status:  planned — update to 'written' once the test file exists"
    );
    Ok(())
}

// ── test list ─────────────────────────────────────────────────────────────────

pub fn list(repo: &Repository, feature_id: Option<&str>, phase_id: Option<&str>) -> Result<()> {
    let manifest = repo.load_manifest()?;
    let tests: Vec<_> = manifest
        .tests
        .iter()
        .filter(|t| {
            feature_id.map_or(true, |f| t.feature_id == f)
                && phase_id.map_or(true, |p| t.phase_id == p)
        })
        .collect();

    if tests.is_empty() {
        println!("No tests found.");
        return Ok(());
    }

    println!(
        "{:<35} {:<20} {:<20} {:<12} {}",
        "ID", "FEATURE", "PHASE", "KIND", "STATUS"
    );
    println!("{}", "─".repeat(100));
    for t in &tests {
        let kind = format!("{:?}", t.kind).to_lowercase();
        let status = format!("{:?}", t.status).to_lowercase();
        println!(
            "{:<35} {:<20} {:<20} {:<12} {}",
            t.id, t.feature_id, t.phase_id, kind, status
        );
    }
    Ok(())
}

// ── test set-status ───────────────────────────────────────────────────────────

pub fn set_status(
    repo: &Repository,
    test_id: &str,
    status: crate::core::models::TestStatus,
) -> Result<()> {
    let mut manifest = repo.load_manifest()?;

    let test = manifest
        .tests
        .iter_mut()
        .find(|t| t.id == test_id)
        .ok_or_else(|| anyhow::anyhow!("test '{}' not found in manifest", test_id))?;

    test.status = status.clone();
    repo.save_manifest(&manifest)?;

    let status_str = format!("{:?}", status).to_lowercase();
    println!("✓ Test '{test_id}' status updated to '{status_str}'.");
    Ok(())
}
