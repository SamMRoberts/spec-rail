use anyhow::Result;
use std::path::Path;

use crate::core::models::{AgentRunResult, AgentTask};
use crate::runtime::process::run_shell;

use super::adapter::AgentAdapter;

/// Generic shell adapter.
///
/// Passes the prompt as the `SPECRAIL_PROMPT` environment variable and runs
/// the command specified in `SPECRAIL_AGENT_CMD` (defaulting to `echo`).
/// In real usage the user would set `SPECRAIL_AGENT_CMD` to a wrapper script
/// that invokes their preferred AI coding CLI.
pub struct GenericShellAdapter {
    /// Shell command to invoke, e.g. `"my-ai-cli code"`
    pub command: String,
}

impl GenericShellAdapter {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

impl AgentAdapter for GenericShellAdapter {
    fn name(&self) -> &str {
        "generic-shell"
    }

    fn run(&self, task: &AgentTask, cwd: Option<&Path>) -> Result<AgentRunResult> {
        // Build allowed/forbidden path hints for the prompt environment
        let allowed = task.allowed_paths.join(":");
        let forbidden = task.forbidden_paths.join(":");

        let full_cmd = format!(
            "SPECRAIL_PROMPT={prompt} SPECRAIL_ALLOWED_PATHS={allowed} \
             SPECRAIL_FORBIDDEN_PATHS={forbidden} {cmd}",
            prompt = shell_escape(&task.prompt),
            cmd = self.command,
        );

        let output = run_shell(&full_cmd, cwd)?;
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

/// Minimal shell-safe single-quote escaping for a prompt string.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
