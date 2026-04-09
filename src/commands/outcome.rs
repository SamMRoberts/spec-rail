use anyhow::{bail, Result};

use crate::core::{
    ledger::Ledger,
    models::{LedgerEvent, LedgerEventType, OutcomeSpec, OutcomeStatus},
    repository::Repository,
};

// ── outcome new ───────────────────────────────────────────────────────────────

pub struct NewArgs {
    pub feature_id: String,
    pub outcome_id: String,
    pub title: String,
    pub goal: String,
    pub order: u32,
    pub prerequisites: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_tests: Vec<String>,
    pub required_test_files: Vec<String>,
}

pub struct EditArgs {
    pub feature_id: String,
    pub outcome_id: String,
    pub title: String,
    pub goal: String,
    pub order: u32,
    pub prerequisites: Vec<String>,
    pub allowed_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub required_tests: Vec<String>,
    pub required_test_files: Vec<String>,
}

pub(crate) fn create(repo: &Repository, args: NewArgs) -> Result<OutcomeSpec> {
    // Ensure the feature exists
    repo.load_feature(&args.feature_id)?;

    if repo.outcome_exists(&args.feature_id, &args.outcome_id)? {
        bail!(
            "outcome '{}' already exists for feature '{}'",
            args.outcome_id,
            args.feature_id
        );
    }

    let outcome = OutcomeSpec {
        id: args.outcome_id.clone(),
        feature_id: args.feature_id.clone(),
        title: args.title,
        goal: args.goal,
        order: args.order,
        prerequisites: args.prerequisites,
        allowed_paths: args.allowed_paths,
        forbidden_paths: args.forbidden_paths,
        required_tests: args.required_tests,
        required_test_files: args.required_test_files,
        status: OutcomeStatus::Pending,
    };

    repo.save_outcome(&outcome)?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeCreated)
        .with_feature(&args.feature_id)
        .with_outcome(&args.outcome_id);
    Ledger::append(repo, &event)?;

    Ok(outcome)
}

pub fn new(repo: &Repository, args: NewArgs) -> Result<()> {
    let outcome = create(repo, args)?;

    println!(
        "✓ Outcome '{}' created for feature '{}'",
        outcome.id, outcome.feature_id
    );
    println!("  Stored in .specrail/specrail.db");
    println!();
    println!("  What each field does:");
    println!("    title  — human-readable label shown in `outcome list` and status");
    println!("    goal   — acceptance criterion: what 'done' looks like for this outcome");
    println!("    order  — sequence position; `specrail advance` steps through outcomes in this order");
    println!("    prereq — outcome IDs that must be verified before this one can be activated");
    println!("    allow  — glob paths the AI agent may modify (omit to allow all paths)");
    println!("    forbid — glob paths the AI agent must NOT touch");
    println!("    test   — required manifest test IDs that must pass");
    println!("    test-file — test file paths used by `specrail test generate`");
    println!();
    println!("  Next steps:");
    println!(
        "    Register a test:  specrail test add <test-id> --feature {} --outcome {} --path <path>",
        outcome.feature_id, outcome.id
    );
    println!(
        "    Activate:         specrail outcome activate {} {}",
        outcome.feature_id, outcome.id
    );
    Ok(())
}

// ── outcome list ──────────────────────────────────────────────────────────────

pub fn list(repo: &Repository, feature_id: &str) -> Result<()> {
    // Ensure feature exists
    repo.load_feature(feature_id)?;

    let outcomes = repo.list_outcomes(feature_id)?;
    if outcomes.is_empty() {
        println!("No outcomes for feature '{feature_id}'. Add one with `specrail outcome new`.");
        return Ok(());
    }

    println!("{:<5} {:<30} {:<12} {}", "ORD", "ID", "STATUS", "TITLE");
    println!("{}", "─".repeat(75));
    for o in &outcomes {
        let status = format!("{:?}", o.status).to_lowercase();
        println!("{:<5} {:<30} {:<12} {}", o.order, o.id, status, o.title);
    }
    Ok(())
}

// ── outcome show ──────────────────────────────────────────────────────────────

pub fn show(repo: &Repository, feature_id: &str, outcome_id: &str) -> Result<()> {
    let o = repo.load_outcome(feature_id, outcome_id)?;
    let feature = repo.load_feature(feature_id)?;
    println!("Outcome: {} (order {})", o.id, o.order);
    println!("  order controls the sequence used by `specrail advance`");
    println!("Feature: {}", o.feature_id);
    println!(
        "Hierarchy: {}/{}/{}",
        feature.solution_id, feature.project_id, feature.component_id
    );
    println!("Title:   {}", o.title);
    println!("Goal:    {}", o.goal);
    println!("  goal is the acceptance criterion — what 'done' looks like for this outcome");
    println!("Status:  {:?}", o.status);

    if !o.prerequisites.is_empty() {
        println!("\nPrerequisites (must be verified before this outcome can be activated):");
        for pr in &o.prerequisites {
            println!("  • {pr}");
        }
    }
    if !o.allowed_paths.is_empty() {
        println!("\nAllowed paths (AI agent may only modify these):");
        for ap in &o.allowed_paths {
            println!("  • {ap}");
        }
    }
    if !o.forbidden_paths.is_empty() {
        println!("\nForbidden paths (AI agent must NOT touch these):");
        for fp in &o.forbidden_paths {
            println!("  • {fp}");
        }
    }
    if !o.required_tests.is_empty() {
        println!("\nRequired test IDs:");
        for rt in &o.required_tests {
            println!("  • {rt}");
        }
        println!("  Tip: add individual tests with `specrail test add`");
    } else {
        println!("\nNo required test IDs set.");
    }
    if !o.required_test_files.is_empty() {
        println!("\nRequired test files (used by `specrail test generate` to create test files):");
        for rt in &o.required_test_files {
            println!("  • {rt}");
        }
    } else {
        println!("\nNo required test file paths set.");
        println!(
            "  Add tests with: specrail test add <id> --feature {} --outcome {} --path <path>",
            o.feature_id, o.id
        );
    }
    Ok(())
}

pub(crate) fn edit_outcome(repo: &Repository, args: EditArgs) -> Result<OutcomeSpec> {
    let mut outcome = repo.load_outcome(&args.feature_id, &args.outcome_id)?;

    outcome.title = args.title;
    outcome.goal = args.goal;
    outcome.order = args.order;
    outcome.prerequisites = args.prerequisites;
    outcome.allowed_paths = args.allowed_paths;
    outcome.forbidden_paths = args.forbidden_paths;
    outcome.required_tests = args.required_tests;
    outcome.required_test_files = args.required_test_files;

    if matches!(
        outcome.status,
        OutcomeStatus::Verified | OutcomeStatus::Failed | OutcomeStatus::Skipped
    ) {
        outcome.status = OutcomeStatus::Pending;
    }

    repo.save_outcome(&outcome)?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeEdited)
        .with_feature(&outcome.feature_id)
        .with_outcome(&outcome.id);
    Ledger::append(repo, &event)?;

    Ok(outcome)
}

pub(crate) fn ensure_required_test(
    repo: &Repository,
    feature_id: &str,
    outcome_id: &str,
    test_id: &str,
) -> Result<bool> {
    let mut outcome = repo.load_outcome(feature_id, outcome_id)?;

    if outcome.required_tests.iter().any(|existing| existing == test_id) {
        return Ok(false);
    }

    outcome.required_tests.push(test_id.to_string());
    repo.save_outcome(&outcome)?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeEdited)
        .with_feature(feature_id)
        .with_outcome(outcome_id)
        .with_message(format!("required test '{}' added", test_id));
    Ledger::append(repo, &event)?;

    Ok(true)
}

pub(crate) struct RequiredTestReferenceUpdate {
    pub added_test_id: bool,
    pub added_test_file: bool,
}

pub(crate) fn ensure_required_test_reference(
    repo: &Repository,
    feature_id: &str,
    outcome_id: &str,
    test_id: &str,
    path: &str,
) -> Result<RequiredTestReferenceUpdate> {
    let mut outcome = repo.load_outcome(feature_id, outcome_id)?;
    let mut added_test_id = false;
    let mut added_test_file = false;

    if !outcome.required_tests.iter().any(|existing| existing == test_id) {
        outcome.required_tests.push(test_id.to_string());
        added_test_id = true;
    }

    if !outcome
        .required_test_files
        .iter()
        .any(|existing| existing == path)
    {
        outcome.required_test_files.push(path.to_string());
        added_test_file = true;
    }

    if added_test_id || added_test_file {
        repo.save_outcome(&outcome)?;

        let mut changes = Vec::new();
        if added_test_id {
            changes.push(format!("required test '{}' added", test_id));
        }
        if added_test_file {
            changes.push(format!("required test file '{}' added", path));
        }

        let event = LedgerEvent::new(LedgerEventType::OutcomeEdited)
            .with_feature(feature_id)
            .with_outcome(outcome_id)
            .with_message(changes.join("; "));
        Ledger::append(repo, &event)?;
    }

    Ok(RequiredTestReferenceUpdate {
        added_test_id,
        added_test_file,
    })
}

pub fn edit(repo: &Repository, args: EditArgs) -> Result<()> {
    let outcome = edit_outcome(repo, args)?;
    println!(
        "✓ Outcome '{}' updated for feature '{}'",
        outcome.id, outcome.feature_id
    );
    Ok(())
}

pub(crate) fn reset_status_to_pending(
    repo: &Repository,
    feature_id: &str,
    outcome_id: &str,
) -> Result<OutcomeSpec> {
    let mut outcome = repo.load_outcome(feature_id, outcome_id)?;

    if matches!(outcome.status, OutcomeStatus::Pending | OutcomeStatus::Active) {
        bail!(
            "outcome '{outcome_id}' is already editable (status: {:?})",
            outcome.status
        );
    }

    outcome.status = OutcomeStatus::Pending;
    repo.save_outcome(&outcome)?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeEdited)
        .with_feature(feature_id)
        .with_outcome(outcome_id);
    Ledger::append(repo, &event)?;

    Ok(outcome)
}

// ── outcome activate ──────────────────────────────────────────────────────────

pub(crate) fn activate_outcome(
    repo: &Repository,
    feature_id: &str,
    outcome_id: &str,
) -> Result<()> {
    let mut outcome = repo.load_outcome(feature_id, outcome_id)?;
    let feature = repo.load_feature(feature_id)?;
    let mut state = repo.load_state()?;

    if outcome.status == OutcomeStatus::Verified {
        bail!("outcome '{outcome_id}' is already verified — use `specrail advance`");
    }

    outcome.status = OutcomeStatus::Active;
    repo.save_outcome(&outcome)?;

    state.active_solution = Some(feature.solution_id);
    state.active_project = Some(feature.project_id);
    state.active_component = Some(feature.component_id);
    state.active_feature = Some(feature_id.to_string());
    state.active_outcome = Some(outcome_id.to_string());
    repo.save_state(&state)?;

    // Also set the current_outcome on the feature record
    super::feature::set_current_outcome(repo, feature_id, Some(outcome_id))?;

    let event = LedgerEvent::new(LedgerEventType::OutcomeActivated)
        .with_feature(feature_id)
        .with_outcome(outcome_id);
    Ledger::append(repo, &event)?;

    Ok(())
}

pub fn activate(repo: &Repository, feature_id: &str, outcome_id: &str) -> Result<()> {
    activate_outcome(repo, feature_id, outcome_id)?;

    println!("✓ Outcome '{outcome_id}' is now active for feature '{feature_id}'.");
    Ok(())
}
