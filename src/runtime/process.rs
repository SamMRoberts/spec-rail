use anyhow::{Context, Result};
use std::process::{Command, Output};

/// Run a shell command and return its output.
pub fn run_command(cmd: &str, args: &[&str], cwd: Option<&std::path::Path>) -> Result<Output> {
    let mut command = Command::new(cmd);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    let output = command
        .output()
        .with_context(|| format!("running command: {cmd} {}", args.join(" ")))?;
    Ok(output)
}

/// Run a shell command string via `sh -c` and return its output.
pub fn run_shell(shell_cmd: &str, cwd: Option<&std::path::Path>) -> Result<Output> {
    let mut command = Command::new("sh");
    command.args(["-c", shell_cmd]);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    let output = command
        .output()
        .with_context(|| format!("running shell command: {shell_cmd}"))?;
    Ok(output)
}
