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
        config["mcpServers"]["specrail"]["env"]["SPECRAIL_MCP_DEBUG_STDERR"],
        "1"
    );
}

#[test]
fn plugin_workflow_stage_skill_exists() {
    let skill_path = format!(
        "{}/.github/plugin/skills/specrail-workflow/SKILL.md",
        env!("CARGO_MANIFEST_DIR")
    );
    let skill = fs::read_to_string(skill_path).unwrap();

    assert!(skill.contains("name: specrail-workflow"));
    assert!(skill.contains("specrail_status"));
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
}
