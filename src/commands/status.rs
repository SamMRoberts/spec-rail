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

    // Active feature/phase
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

    match &state.active_phase {
        None => println!("Active phase:   (none)"),
        Some(pid) => {
            println!("Active phase:   {pid}");
            if let Some(fid) = &state.active_feature {
                if let Ok(phase) = repo.load_phase(fid, pid) {
                    println!("  Goal:   {}", phase.goal);
                    println!("  Status: {:?}", phase.status);
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
        let phases = repo.list_phases(&f.id)?;
        let verified = phases
            .iter()
            .filter(|p| p.status == crate::core::models::PhaseStatus::Verified)
            .count();
        println!(
            "  [{status:<8}] {} — {}/{} phases verified",
            f.id,
            verified,
            phases.len()
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
