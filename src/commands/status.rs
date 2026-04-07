use anyhow::Result;

use crate::core::repository::Repository;

pub fn run(repo: &Repository) -> Result<()> {
    let state = repo.load_state()?;
    let config = repo.load_config()?;

    println!("specrail status");
    println!("{}", "═".repeat(50));
    println!("Project:       {}", config.name);
    println!("Test command:  {}", config.test_command);
    println!("Default agent: {}", config.default_agent);
    println!();

    // Active feature/outcome
    match &state.active_feature {
        None => println!("Active feature: (none)"),
        Some(fid) => {
            println!("Active feature: {fid}");
            if let Ok(feature) = repo.load_feature(fid) {
                println!("  Title:  {}", feature.title);
                println!("  Status: {:?}", feature.status);
            }
        }
    }

    match &state.active_outcome {
        None => println!("Active outcome:   (none)"),
        Some(oid) => {
            println!("Active outcome:   {oid}");
            if let Some(fid) = &state.active_feature {
                if let Ok(outcome) = repo.load_outcome(fid, oid) {
                    println!("  Goal:   {}", outcome.goal);
                    println!("  Status: {:?}", outcome.status);
                }
            }
        }
    }

    println!();

    // Features summary
    let features = repo.list_features()?;
    println!("Features: {}", features.len());
    for f in &features {
        let status = format!("{:?}", f.status).to_lowercase();
        let outcomes = repo.list_outcomes(&f.id)?;
        let verified = outcomes
            .iter()
            .filter(|o| o.status == crate::core::models::OutcomeStatus::Verified)
            .count();
        println!(
            "  [{status:<8}] {} — {}/{} outcomes verified",
            f.id,
            verified,
            outcomes.len()
        );
    }

    // Manifest summary
    let manifest = repo.load_manifest()?;
    println!();
    println!("Tests in manifest: {}", manifest.tests.len());
    let passing = manifest
        .tests
        .iter()
        .filter(|t| t.status == crate::core::models::TestStatus::Passing)
        .count();
    let written = manifest
        .tests
        .iter()
        .filter(|t| t.status == crate::core::models::TestStatus::Written)
        .count();
    let planned = manifest
        .tests
        .iter()
        .filter(|t| t.status == crate::core::models::TestStatus::Planned)
        .count();
    println!("  passing: {passing}  written: {written}  planned: {planned}");

    Ok(())
}
