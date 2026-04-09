use std::fs;

use serde_json::Value;

#[test]
fn plugin_manifest_points_to_skill_and_mcp_config() {
    let plugin_manifest = fs::read_to_string(format!(
        "{}/.github/plugin/plugin.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let manifest: Value = serde_json::from_str(&plugin_manifest).unwrap();

    assert_eq!(manifest["name"], "specrail");
    assert_eq!(manifest["skills"], "skills/");
    assert_eq!(manifest["mcpServers"], ".mcp.json");
}

#[test]
fn plugin_mcp_config_runs_specrail_mcp_server() {
    let mcp_config = fs::read_to_string(format!(
        "{}/.github/plugin/.mcp.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let config: Value = serde_json::from_str(&mcp_config).unwrap();

    assert_eq!(config["mcpServers"]["specrail"]["type"], "stdio");
    assert_eq!(config["mcpServers"]["specrail"]["command"], "specrail");
    assert_eq!(config["mcpServers"]["specrail"]["args"], serde_json::json!(["mcp-server"]));
    assert_eq!(
        config["mcpServers"]["specrail"]["env"]["SPECRAIL_MCP_LOG_LEVEL"],
        "warning"
    );
    // The packaged plugin must not enable debug stderr by default.
    assert!(
        config["mcpServers"]["specrail"]["env"]["SPECRAIL_MCP_DEBUG_STDERR"].is_null(),
        "SPECRAIL_MCP_DEBUG_STDERR must not be set in the packaged plugin .mcp.json"
    );
}

#[test]
fn workspace_mcp_config_uses_specrail_server_id() {
    let mcp_config = fs::read_to_string(format!(
        "{}/.vscode/mcp.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();

    assert!(mcp_config.contains("\"specrail\""));
    assert!(mcp_config.contains("\"type\": \"stdio\""));
    assert!(mcp_config.contains("\"command\": \"${workspaceFolder}/target/debug/specrail\""));
    assert!(mcp_config.contains("\"mcp-server\""));
    assert!(mcp_config.contains("\"SPECRAIL_MCP_LOG_LEVEL\": \"verbose\""));
    assert!(mcp_config.contains("\"SPECRAIL_MCP_DEBUG_STDERR\": \"1\""));
}

#[test]
fn plugin_workflow_stage_skill_exists() {
    let skill_path = format!(
        "{}/.github/plugin/skills/specrail-plan-features/SKILL.md",
        env!("CARGO_MANIFEST_DIR")
    );
    let skill = fs::read_to_string(skill_path).unwrap();

    assert!(skill.contains("name: specrail-plan-features"));
    assert!(skill.contains("specrail_status"));
    assert!(skill.contains("platform, language, stack/framework, interface type, and deployment/runtime target"));
    assert!(skill.contains("ask whether to create it or correct the reference"));
}

#[test]
fn plugin_tdd_skill_exists() {
    let skill_path = format!(
        "{}/.github/plugin/skills/specrail-tdd/SKILL.md",
        env!("CARGO_MANIFEST_DIR")
    );
    let skill = fs::read_to_string(skill_path).unwrap();

    assert!(skill.contains("name: specrail-tdd"));
    assert!(skill.contains("workflow.recommended_skill"));
    assert!(skill.contains("specrail_status"));
    assert!(skill.contains("Only create or generate tests"));
    assert!(skill.contains("ask targeted follow-up questions instead of guessing"));
    assert!(skill.contains("warn the user and ask whether to refine"));
}

#[test]
fn workspace_specrail_agent_exists_and_references_workflow_guidance() {
    let agent_path = format!(
        "{}/.github/agents/specrail.agent.md",
        env!("CARGO_MANIFEST_DIR")
    );
    let agent = fs::read_to_string(agent_path).unwrap();

    assert!(agent.contains("name: Specrail"));
    assert!(agent.contains("description:"));
    assert!(agent.contains("specrail_status"));
    assert!(agent.contains("workflow.recommended_skill"));
    assert!(agent.contains("Do not create tests that are not specified"));
    assert!(agent.contains("Do not write more production code"));
    assert!(agent.contains("determine whether they are starting a new project from scratch"));
    assert!(agent.contains("ask for the missing information before proceeding"));
    assert!(agent.contains("ask for clarification instead of guessing"));
    assert!(agent.contains("ask the user whether they want to create it or correct the reference"));
    assert!(agent.contains("warn the user and ask whether they want to continue as-is or refine it"));
}
