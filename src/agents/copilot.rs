use anyhow::Result;
use std::path::Path;

use crate::core::models::{AgentRunResult, AgentTask};
use crate::runtime::process::run_command;

use super::adapter::AgentAdapter;

/// Adapter for the GitHub Copilot CLI (`gh copilot suggest`).
///
/// The prompt is written to a temporary file and passed to the CLI.  
/// Requires `gh` with the `copilot` extension installed.
pub struct CopilotAdapter;

impl AgentAdapter for CopilotAdapter {
    fn name(&self) -> &str {
        "copilot"
    }

    fn run(&self, task: &AgentTask, cwd: Option<&Path>) -> Result<AgentRunResult> {
        // gh copilot suggest -t shell "<prompt>"
        let output = run_command(
            "gh",
            &["copilot", "suggest", "-t", "shell", &task.prompt],
            cwd,
        )?;

        let exit_code = output.status.code();
        let success = output.status.success();

        Ok(AgentRunResult {
            success,
            exit_code,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}
