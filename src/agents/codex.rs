use anyhow::Result;
use std::path::Path;

use crate::core::models::{AgentRunResult, AgentTask};
use crate::runtime::process::run_shell;

use super::adapter::AgentAdapter;

/// Adapter for the OpenAI Codex CLI (`codex`).
///
/// Passes the prompt as a positional argument.  
/// Requires the `codex` binary to be on `PATH`.
pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn name(&self) -> &str {
        "codex"
    }

    fn run(&self, task: &AgentTask, cwd: Option<&Path>) -> Result<AgentRunResult> {
        let cmd = format!("codex {}", shell_quote(&task.prompt));
        let output = run_shell(&cmd, cwd)?;

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

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
