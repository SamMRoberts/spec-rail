pub mod adapter;
pub mod codex;
pub mod copilot;
pub mod generic_shell;

use anyhow::{bail, Result};

use crate::core::models::AgentTask;

use self::{
    adapter::AgentAdapter,
    codex::CodexAdapter,
    copilot::CopilotAdapter,
    generic_shell::GenericShellAdapter,
};

/// Resolve an agent adapter by name.
///
/// `generic-shell` accepts an optional `command` override via the
/// `SPECRAIL_AGENT_CMD` environment variable (defaults to `echo`).
pub fn resolve_adapter(name: &str) -> Result<Box<dyn AgentAdapter>> {
    match name {
        "generic-shell" => {
            let cmd = std::env::var("SPECRAIL_AGENT_CMD").unwrap_or_else(|_| "echo".into());
            Ok(Box::new(GenericShellAdapter::new(cmd)))
        }
        "copilot" => Ok(Box::new(CopilotAdapter)),
        "codex" => Ok(Box::new(CodexAdapter)),
        other => bail!("unknown agent '{other}' — supported: generic-shell, copilot, codex"),
    }
}

/// Run an agent task using the named adapter, resolved automatically.
pub fn run_task(
    agent_name: &str,
    task: &AgentTask,
    cwd: Option<&std::path::Path>,
) -> Result<crate::core::models::AgentRunResult> {
    let adapter = resolve_adapter(agent_name)?;
    adapter.run(task, cwd)
}
