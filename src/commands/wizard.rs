use std::io::{self, BufRead, Write};

use anyhow::{bail, Result};

use crate::{
    commands::{feature, phase, test},
    core::repository::Repository,
};

pub fn run(repo: &Repository) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    run_with_io(repo, &mut reader, &mut writer)
}

fn run_with_io<R: BufRead, W: Write>(
    repo: &Repository,
    reader: &mut R,
    writer: &mut W,
) -> Result<()> {
    writeln!(writer, "\nWalkthrough: add features and phases to get started.")?;
    writeln!(
        writer,
        "Press Enter on an empty list prompt to move to the next section."
    )?;
    writer.flush()?;

    let mut created_any = false;

    loop {
        let Some(feature_args) = prompt_feature(reader, writer, created_any)? else {
            if !created_any {
                writeln!(
                    writer,
                    "Walkthrough skipped. Continue with `specrail feature new ...`."
                )?;
            }
            break;
        };

        let feature = match feature::create(repo, feature_args) {
            Ok(feature) => feature,
            Err(error) => {
                writeln!(writer, "Could not create feature: {error}")?;
                writer.flush()?;
                continue;
            }
        };

        writeln!(writer, "\nCreated feature '{}'.", feature.id)?;

        let latest_phase_id = prompt_phases(repo, reader, writer, &feature.id)?;

        feature::activate_feature(repo, &feature.id)?;
        phase::activate_phase(repo, &feature.id, &latest_phase_id)?;

        writeln!(
            writer,
            "Activated feature '{}' and phase '{}'.",
            feature.id, latest_phase_id
        )?;
        writer.flush()?;

        created_any = true;
        if !confirm(reader, writer, "Add another feature?", false)? {
            break;
        }
    }

    if created_any {
        if confirm(reader, writer, "Generate required tests with Copilot now?", true)? {
            match test::generate(repo, Some("copilot")) {
                Ok(()) => {
                    writeln!(writer, "Required tests generated and registered.")?;
                }
                Err(error) => {
                    writeln!(writer, "Test generation skipped: {error}")?;
                }
            }
        }

        writeln!(writer, "\nContinue with `specrail implement`, `specrail verify`, and `specrail advance`.")?;
    }
    writer.flush()?;

    Ok(())
}

fn prompt_feature<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    already_created_feature: bool,
) -> Result<Option<feature::NewArgs>> {
    let feature_id_prompt = if already_created_feature {
        "\nFeature ID (leave blank to stop adding features): "
    } else {
        "\nFeature ID (leave blank to skip setup): "
    };

    let Some(id) = prompt_optional(reader, writer, feature_id_prompt)? else {
        return Ok(None);
    };

    let title = prompt_required(reader, writer, "Feature title: ")?;
    let purpose = prompt_required(reader, writer, "Feature purpose: ")?;
    let outcomes = collect_list(reader, writer, "Feature outcome")?;
    let constraints = collect_list(reader, writer, "Feature constraint")?;
    let non_goals = collect_list(reader, writer, "Feature non-goal")?;
    let dependencies = collect_list(reader, writer, "Feature dependency")?;

    Ok(Some(feature::NewArgs {
        id,
        title,
        purpose,
        outcomes,
        constraints,
        non_goals,
        dependencies,
    }))
}

fn prompt_phases<R: BufRead, W: Write>(
    repo: &Repository,
    reader: &mut R,
    writer: &mut W,
    feature_id: &str,
) -> Result<String> {
    let mut latest_phase_id = None;
    let mut next_order = 1;

    loop {
        let phase_args = match prompt_phase(reader, writer, feature_id, next_order, latest_phase_id.is_some())? {
            Some(args) => args,
            None if latest_phase_id.is_some() => break,
            None => {
                writeln!(writer, "At least one phase is required for each feature.")?;
                writer.flush()?;
                continue;
            }
        };

        let phase = match phase::create(repo, phase_args) {
            Ok(phase) => phase,
            Err(error) => {
                writeln!(writer, "Could not create phase: {error}")?;
                writer.flush()?;
                continue;
            }
        };

        writeln!(writer, "Created phase '{}' (order {}).", phase.id, phase.order)?;
        writer.flush()?;

        next_order = phase.order.saturating_add(1);
        latest_phase_id = Some(phase.id.clone());

        let question = format!("Add another phase for '{}' ?", feature_id);
        if !confirm(reader, writer, &question, false)? {
            break;
        }
    }

    latest_phase_id.ok_or_else(|| anyhow::anyhow!("walkthrough ended without a phase"))
}

fn prompt_phase<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    feature_id: &str,
    default_order: u32,
    already_created_phase: bool,
) -> Result<Option<phase::NewArgs>> {
    let phase_id_prompt = if already_created_phase {
        "\nPhase ID (leave blank to stop adding phases): "
    } else {
        "\nPhase ID: "
    };

    let Some(phase_id) = prompt_optional(reader, writer, phase_id_prompt)? else {
        return Ok(None);
    };

    let title = prompt_required(reader, writer, "Phase title: ")?;
    let goal = prompt_required(reader, writer, "Phase goal: ")?;
    let order = prompt_u32_with_default(
        reader,
        writer,
        &format!("Phase order [{default_order}]: "),
        default_order,
    )?;
    let prerequisites = collect_list(reader, writer, "Phase prerequisite")?;
    let allowed_paths = collect_list(reader, writer, "Allowed path")?;
    let forbidden_paths = collect_list(reader, writer, "Forbidden path")?;
    let required_tests = collect_list(reader, writer, "Required test path")?;

    Ok(Some(phase::NewArgs {
        feature_id: feature_id.to_string(),
        phase_id,
        title,
        goal,
        order,
        prerequisites,
        allowed_paths,
        forbidden_paths,
        required_tests,
    }))
}

fn collect_list<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    label: &str,
) -> Result<Vec<String>> {
    let mut values = Vec::new();

    loop {
        let prompt = format!("{label} (leave blank when finished): ");
        match prompt_input(reader, writer, &prompt)? {
            Some(value) if !value.is_empty() => values.push(value),
            Some(_) | None => return Ok(values),
        }
    }
}

fn prompt_required<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    prompt: &str,
) -> Result<String> {
    loop {
        match prompt_input(reader, writer, prompt)? {
            Some(value) if !value.is_empty() => return Ok(value),
            Some(_) => {
                writeln!(writer, "A value is required.")?;
                writer.flush()?;
            }
            None => bail!("walkthrough input ended unexpectedly"),
        }
    }
}

fn prompt_optional<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    prompt: &str,
) -> Result<Option<String>> {
    match prompt_input(reader, writer, prompt)? {
        Some(value) if value.is_empty() => Ok(None),
        Some(value) => Ok(Some(value)),
        None => Ok(None),
    }
}

fn prompt_u32_with_default<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    prompt: &str,
    default: u32,
) -> Result<u32> {
    loop {
        match prompt_input(reader, writer, prompt)? {
            Some(value) if value.is_empty() => return Ok(default),
            Some(value) => match value.parse::<u32>() {
                Ok(parsed) => return Ok(parsed),
                Err(_) => {
                    writeln!(writer, "Enter a positive integer.")?;
                    writer.flush()?;
                }
            },
            None => return Ok(default),
        }
    }
}

fn confirm<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    question: &str,
    default: bool,
) -> Result<bool> {
    let suffix = if default { "[Y/n]" } else { "[y/N]" };

    loop {
        let prompt = format!("{question} {suffix}: ");
        match prompt_input(reader, writer, &prompt)? {
            Some(value) if value.is_empty() => return Ok(default),
            Some(value) => match value.to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => {
                    writeln!(writer, "Please answer yes or no.")?;
                    writer.flush()?;
                }
            },
            None => return Ok(default),
        }
    }
}

fn prompt_input<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    prompt: &str,
) -> Result<Option<String>> {
    write!(writer, "{prompt}")?;
    writer.flush()?;

    let mut line = String::new();
    let bytes_read = reader.read_line(&mut line)?;
    if bytes_read == 0 {
        return Ok(None);
    }

    Ok(Some(line.trim().to_string()))
}