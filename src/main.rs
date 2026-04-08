mod agents;
mod cli;
mod commands;
mod core;
mod errors;
mod mcp;
mod policy;
mod prompts;
mod runtime;

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

use cli::{
    Cli, Commands, ComponentCommands, FeatureCommands, OutcomeCommands, ProjectCommands,
    SolutionCommands, TestCommands,
};
use core::repository::Repository;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Commands::McpServer = cli.command {
        return mcp::run();
    }

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
        .with_writer(std::io::stderr)
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
    repo.ensure_hierarchy()?;

    match cli.command {
        Commands::Init { .. } => unreachable!(),

        Commands::Solution(sub) => match sub {
            SolutionCommands::New { id, title, purpose } => {
                commands::solution::new(&repo, commands::solution::NewArgs { id, title, purpose })
            }
            SolutionCommands::List => commands::solution::list(&repo),
            SolutionCommands::Show { id } => commands::solution::show(&repo, &id),
            SolutionCommands::Edit { id, title, purpose } => commands::solution::edit(
                &repo,
                commands::solution::EditArgs { id, title, purpose },
            ),
            SolutionCommands::Activate { id } => commands::solution::activate(&repo, &id),
        },

        Commands::Project(sub) => match sub {
            ProjectCommands::New {
                solution_id,
                id,
                title,
                purpose,
            } => commands::project::new(
                &repo,
                commands::project::NewArgs {
                    solution_id,
                    id,
                    title,
                    purpose,
                },
            ),
            ProjectCommands::List { solution_id } => commands::project::list(&repo, &solution_id),
            ProjectCommands::Show { id } => commands::project::show(&repo, &id),
            ProjectCommands::Edit { id, title, purpose } => commands::project::edit(
                &repo,
                commands::project::EditArgs { id, title, purpose },
            ),
            ProjectCommands::Activate { id } => commands::project::activate(&repo, &id),
        },

        Commands::Component(sub) => match sub {
            ComponentCommands::New {
                project_id,
                id,
                title,
                purpose,
            } => commands::component::new(
                &repo,
                commands::component::NewArgs {
                    project_id,
                    id,
                    title,
                    purpose,
                },
            ),
            ComponentCommands::List { project_id } => {
                commands::component::list(&repo, &project_id)
            }
            ComponentCommands::Show { id } => commands::component::show(&repo, &id),
            ComponentCommands::Edit { id, title, purpose } => commands::component::edit(
                &repo,
                commands::component::EditArgs { id, title, purpose },
            ),
            ComponentCommands::Activate { id } => commands::component::activate(&repo, &id),
        },

        Commands::Feature(sub) => match sub {
            FeatureCommands::New {
                id,
                component,
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
                    component_id: component,
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
            FeatureCommands::Edit {
                id,
                title,
                purpose,
                outcomes,
                constraints,
                non_goals,
                dependencies,
            } => commands::feature::edit(
                &repo,
                commands::feature::EditArgs {
                    id,
                    title,
                    purpose,
                    outcomes,
                    constraints,
                    non_goals,
                    dependencies,
                },
            ),
            FeatureCommands::Activate { id } => commands::feature::activate(&repo, &id),
        },

        Commands::Outcome(sub) => match sub {
            OutcomeCommands::New {
                feature_id,
                outcome_id,
                title,
                goal,
                order,
                prerequisites,
                allowed_paths,
                forbidden_paths,
                required_tests,
                required_test_files,
            } => commands::outcome::new(
                &repo,
                commands::outcome::NewArgs {
                    feature_id,
                    outcome_id,
                    title,
                    goal,
                    order,
                    prerequisites,
                    allowed_paths,
                    forbidden_paths,
                    required_tests,
                    required_test_files,
                },
            ),
            OutcomeCommands::List { feature_id } => commands::outcome::list(&repo, &feature_id),
            OutcomeCommands::Show {
                feature_id,
                outcome_id,
            } => commands::outcome::show(&repo, &feature_id, &outcome_id),
            OutcomeCommands::Edit {
                feature_id,
                outcome_id,
                title,
                goal,
                order,
                prerequisites,
                allowed_paths,
                forbidden_paths,
                required_tests,
                required_test_files,
            } => commands::outcome::edit(
                &repo,
                commands::outcome::EditArgs {
                    feature_id,
                    outcome_id,
                    title,
                    goal,
                    order,
                    prerequisites,
                    allowed_paths,
                    forbidden_paths,
                    required_tests,
                    required_test_files,
                },
            ),
            OutcomeCommands::Activate {
                feature_id,
                outcome_id,
            } => commands::outcome::activate(&repo, &feature_id, &outcome_id),
        },

        Commands::Test(sub) => match sub {
            TestCommands::Add {
                id,
                name,
                feature,
                outcome,
                path,
                kind,
                purpose_refs,
            } => {
                let test_kind = parse_test_kind(&kind)?;
                commands::test::add(
                    &repo,
                    commands::test::AddArgs {
                        id,
                        name,
                        feature_id: feature,
                        outcome_id: outcome,
                        path,
                        kind: test_kind,
                        purpose_refs,
                    },
                )
            }
            TestCommands::Generate { agent } => {
                commands::test::generate(&repo, agent.as_deref())
            }
            TestCommands::Suggest {
                feature,
                outcome,
                agent,
            } => commands::test::suggest(
                &repo,
                feature.as_deref(),
                outcome.as_deref(),
                agent.as_deref(),
            ),
            TestCommands::List { feature, outcome } => {
                commands::test::list(&repo, feature.as_deref(), outcome.as_deref())
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
        Commands::McpServer => unreachable!(),
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
