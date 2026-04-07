use anyhow::Result;

use crate::core::models::{AgentRunResult, AgentTask};

/// Trait implemented by all AI coding agent adapters.
#[allow(dead_code)]
pub trait AgentAdapter: Send + Sync {
    /// Canonical name of the agent (matches `default_agent` in `project.yaml`).
    fn name(&self) -> &str;

    /// Execute the agent with the given task and return the run result.
    fn run(&self, task: &AgentTask, cwd: Option<&std::path::Path>) -> Result<AgentRunResult>;
}
