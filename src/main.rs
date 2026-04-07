mod agents;
mod cli;
mod commands;
mod core;
mod errors;
mod policy;
mod prompts;
mod runtime;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use cli::{
    Cli, Commands, FeatureCommands, PhaseCommands, TestCommands,
};
use core::repository::Repository;

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialise tracing based on verbosity flag
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .compact()
        .init();

    run(cli)
}

fn run(cli: Cli) -> Result<()> {
    // `init` is special — it does not need an existing project
    if let Commands::Init { no_wizard } = &cli.command {
        let cwd = std::env::current_dir()?;
        let repo = Repository::new(&cwd);
        let outcome = commands::init::run(&repo)?;
        if !*no_wizard && outcome.should_start_wizard {
            commands::wizard::run(&repo)?;
        }
        return Ok(());
    }

    // All other commands need an existing project
    let cwd = std::env::current_dir()?;
    let repo = Repository::discover(&cwd)
        .context("could not find a specrail project — run `specrail init` first")?;

    match cli.command {
        Commands::Init { .. } => unreachable!(),

        Commands::Feature(sub) => match sub {
            FeatureCommands::New {
                id,
                title,
                purpose,
                outcomes,
                constraints,
                non_goals,
                dependencies,
            } => commands::feature::new(
                &repo,
                commands::feature::NewArgs {
                    id,
                    title,
                    purpose,
                    outcomes,
                    constraints,
                    non_goals,
                    dependencies,
                },
            ),
            FeatureCommands::List => commands::feature::list(&repo),
            FeatureCommands::Show { id } => commands::feature::show(&repo, &id),
            FeatureCommands::Activate { id } => commands::feature::activate(&repo, &id),
        },

        Commands::Phase(sub) => match sub {
            PhaseCommands::New {
                feature_id,
                phase_id,
                title,
                goal,
                order,
                prerequisites,
                allowed_paths,
                forbidden_paths,
                required_tests,
            } => commands::phase::new(
                &repo,
                commands::phase::NewArgs {
                    feature_id,
                    phase_id,
                    title,
                    goal,
                    order,
                    prerequisites,
                    allowed_paths,
                    forbidden_paths,
                    required_tests,
                },
            ),
            PhaseCommands::List { feature_id } => commands::phase::list(&repo, &feature_id),
            PhaseCommands::Show {
                feature_id,
                phase_id,
            } => commands::phase::show(&repo, &feature_id, &phase_id),
            PhaseCommands::Activate {
                feature_id,
                phase_id,
            } => commands::phase::activate(&repo, &feature_id, &phase_id),
        },

        Commands::Test(sub) => match sub {
            TestCommands::Add {
                id,
                feature,
                phase,
                path,
                kind,
                purpose_refs,
            } => {
                let test_kind = parse_test_kind(&kind)?;
                commands::test::add(
                    &repo,
                    commands::test::AddArgs {
                        id,
                        feature_id: feature,
                        phase_id: phase,
                        path,
                        kind: test_kind,
                        purpose_refs,
                    },
                )
            }
            TestCommands::List { feature, phase } => {
                commands::test::list(&repo, feature.as_deref(), phase.as_deref())
            }
            TestCommands::SetStatus { id, status } => {
                let test_status = parse_test_status(&status)?;
                commands::test::set_status(&repo, &id, test_status)
            }
        },

        Commands::Implement { agent } => commands::implement::run(&repo, agent.as_deref()),
        Commands::Verify => commands::verify::run(&repo),
        Commands::Advance => commands::advance::run(&repo),
        Commands::Status => commands::status::run(&repo),
        Commands::Trace { limit } => commands::trace::run(&repo, limit),
    }
}

fn parse_test_kind(s: &str) -> Result<core::models::TestKind> {
    use core::models::TestKind;
    match s.to_lowercase().as_str() {
        "unit" => Ok(TestKind::Unit),
        "integration" => Ok(TestKind::Integration),
        "e2e" => Ok(TestKind::E2e),
        other => anyhow::bail!("unknown test kind '{other}' — use: unit | integration | e2e"),
    }
}

fn parse_test_status(s: &str) -> Result<core::models::TestStatus> {
    use core::models::TestStatus;
    match s.to_lowercase().as_str() {
        "planned" => Ok(TestStatus::Planned),
        "written" => Ok(TestStatus::Written),
        "passing" => Ok(TestStatus::Passing),
        "failing" => Ok(TestStatus::Failing),
        other => {
            anyhow::bail!(
                "unknown test status '{other}' — use: planned | written | passing | failing"
            )
        }
    }
}
