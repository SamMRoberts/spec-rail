use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::core::{
    ledger::Ledger,
    models::{FeatureSpec, OutcomeSpec, OutcomeStatus, ProjectState, TestManifest, TestSpec, TestStatus},
    repository::Repository,
};
use crate::commands;

const DEFAULT_PROTOCOL_VERSION: &str = "2025-11-25";
const UI_EXTENSION_NAME: &str = "io.modelcontextprotocol/ui";
const UI_RESOURCE_MIME_TYPE: &str = "text/html;profile=mcp-app";
const FEATURE_NAVIGATE_APP_URI: &str = "ui://specrail/feature-navigate";
const FEATURE_NAVIGATE_APP_HTML: &str = include_str!("mcp_feature_navigate_app.html");
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &[
    DEFAULT_PROTOCOL_VERSION,
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
];

fn negotiate_protocol_version(requested: Option<&str>) -> &'static str {
    SUPPORTED_PROTOCOL_VERSIONS
        .iter()
        .copied()
        .find(|version| Some(*version) == requested)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION)
}

fn server_capabilities() -> Value {
    json!({
        "tools": {
            "listChanged": false
        },
        "resources": {
            "listChanged": false
        },
        "extensions": {
            UI_EXTENSION_NAME: {
                "mimeTypes": [UI_RESOURCE_MIME_TYPE]
            }
        }
    })
}

fn mcp_debug_log(message: impl AsRef<str>) {
    let message = message.as_ref();

    let log_to_stderr = std::env::var_os("SPECRAIL_MCP_DEBUG_STDERR").is_some();
    let log_path = std::env::var_os("SPECRAIL_MCP_DEBUG_LOG");

    if !log_to_stderr && log_path.is_none() {
        return;
    }

    if log_to_stderr {
        let mut stderr = std::io::stderr().lock();
        let _ = writeln!(stderr, "[specrail-mcp] {message}");
    }

    let Some(path) = log_path else {
        return;
    };

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };

    let _ = writeln!(file, "{message}");
}

fn summarize_for_log(text: &str) -> String {
    const LIMIT: usize = 400;
    if text.len() <= LIMIT {
        text.to_string()
    } else {
        let truncated: String = text.chars().take(LIMIT).collect();
        format!("{truncated}…")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MessageTransport {
    ContentLength,
    RawJsonLine,
}

pub fn run() -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut server = McpServer::default();

    mcp_debug_log(format!(
        "server start pid={} cwd={}",
        std::process::id(),
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string())
    ));

    while let Some((message, transport)) = read_message(&mut reader)? {
        mcp_debug_log(format!("received message: {}", summarize_for_log(&serde_json::to_string(&message).unwrap_or_else(|e| format!("<serialization error: {e}>")))));
        if let Some(response) = server.handle_message(message) {
            mcp_debug_log(format!("sending response: {}", summarize_for_log(&serde_json::to_string(&response).unwrap_or_else(|e| format!("<serialization error: {e}>")))));
            write_message(&mut writer, &response, transport)?;
            writer.flush()?;
        }

        if server.should_exit {
            break;
        }
    }

    Ok(())
}

#[derive(Default)]
struct McpServer {
    initialized: bool,
    should_exit: bool,
    protocol_version: String,
}

impl McpServer {
    fn handle_message(&mut self, message: Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = match message.get("method").and_then(Value::as_str) {
            Some(method) => method,
            None => {
                return id.map(|id| jsonrpc_error(id, -32600, "missing method"));
            }
        };

        mcp_debug_log(format!("handle_message method={method}"));

        let response = match method {
            "initialize" => {
                let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
                let requested_protocol = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");
                let client_name = params
                    .get("clientInfo")
                    .and_then(|client| client.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");
                let client_version = params
                    .get("clientInfo")
                    .and_then(|client| client.get("version"))
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>");
                mcp_debug_log(format!(
                    "initialize start id={} requested_protocol={} client={}/{} params={}",
                    id.as_ref()
                        .map(Value::to_string)
                        .unwrap_or_else(|| "<notification>".to_string()),
                    requested_protocol,
                    client_name,
                    client_version,
                    summarize_for_log(&params.to_string())
                ));
                let protocol_version = negotiate_protocol_version(
                    params.get("protocolVersion").and_then(Value::as_str),
                )
                .to_string();
                self.protocol_version = protocol_version.clone();
                mcp_debug_log(format!(
                    "initialize negotiated protocol_version={} supported={:?}",
                    protocol_version,
                    SUPPORTED_PROTOCOL_VERSIONS
                ));

                id.map(|id| {
                    let response = jsonrpc_result(
                        id,
                        json!({
                            "protocolVersion": protocol_version,
                            "capabilities": server_capabilities(),
                            "serverInfo": {
                                "name": "specrail",
                                "version": env!("CARGO_PKG_VERSION")
                            }
                        }),
                    );
                    mcp_debug_log(format!(
                        "initialize response ready id={} body={}",
                        response["id"],
                        summarize_for_log(&response.to_string())
                    ));
                    response
                })
            }
            "notifications/initialized" => {
                self.initialized = true;
                mcp_debug_log("notifications/initialized received; session marked initialized");
                None
            }
            "ping" => id.map(|id| jsonrpc_result(id, json!({}))),
            "tools/list" => id.map(|id| jsonrpc_result(id, json!({ "tools": tool_definitions() }))),
            "resources/list" => {
                id.map(|id| jsonrpc_result(id, json!({ "resources": resource_definitions() })))
            }
            "resources/read" => id.map(|id| match read_resource(message.get("params")) {
                Ok(result) => jsonrpc_result(id, result),
                Err(error) => jsonrpc_error(id, -32002, error.to_string()),
            }),
            "tools/call" => {
                let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
                let result = handle_tool_call(&params).unwrap_or_else(|error| tool_error_payload(error));
                id.map(|id| jsonrpc_result(id, result))
            }
            "shutdown" => id.map(|id| jsonrpc_result(id, json!({}))),
            "exit" => {
                self.should_exit = true;
                None
            }
            _ => id.map(|id| jsonrpc_error(id, -32601, format!("method '{method}' not found"))),
        };

        if method == "shutdown" {
            self.should_exit = false;
        }

        response
    }
}

fn handle_tool_call(params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .context("missing tool name")?;
    let arguments = match params.get("arguments") {
        Some(Value::Object(map)) => map,
        Some(_) => bail!("tool arguments must be an object"),
        None => bail!("missing tool arguments"),
    };

    match name {
        "specrail_status" => tool_status(arguments),
        "specrail_feature_navigate" => tool_feature_navigate(arguments),
        "specrail_solution_list" => tool_solution_list(arguments),
        "specrail_solution_show" => tool_solution_show(arguments),
        "specrail_project_list" => tool_project_list(arguments),
        "specrail_project_show" => tool_project_show(arguments),
        "specrail_component_list" => tool_component_list(arguments),
        "specrail_component_show" => tool_component_show(arguments),
        "specrail_feature_list" => tool_feature_list(arguments),
        "specrail_feature_show" => tool_feature_show(arguments),
        "specrail_outcome_list" => tool_outcome_list(arguments),
        "specrail_outcome_show" => tool_outcome_show(arguments),
        "specrail_outcome_test_review" => tool_outcome_test_review(arguments),
        "specrail_test_list" => tool_test_list(arguments),
        "specrail_trace" => tool_trace(arguments),
        "specrail_init" => tool_init(arguments),
        "specrail_solution_new" => tool_solution_new(arguments),
        "specrail_solution_activate" => tool_solution_activate(arguments),
        "specrail_solution_edit" => tool_solution_edit(arguments),
        "specrail_project_new" => tool_project_new(arguments),
        "specrail_project_activate" => tool_project_activate(arguments),
        "specrail_project_edit" => tool_project_edit(arguments),
        "specrail_component_new" => tool_component_new(arguments),
        "specrail_component_activate" => tool_component_activate(arguments),
        "specrail_component_edit" => tool_component_edit(arguments),
        "specrail_feature_new" => tool_feature_new(arguments),
        "specrail_feature_activate" => tool_feature_activate(arguments),
        "specrail_feature_edit" => tool_feature_edit(arguments),
        "specrail_outcome_new" => tool_outcome_new(arguments),
        "specrail_outcome_activate" => tool_outcome_activate(arguments),
        "specrail_outcome_edit" => tool_outcome_edit(arguments),
        "specrail_outcome_add_required_test" => tool_outcome_add_required_test(arguments),
        "specrail_outcome_unverify" => tool_outcome_unverify(arguments),
        "specrail_test_add" => tool_test_add(arguments),
        "specrail_test_suggest" => tool_test_suggest(arguments),
        "specrail_test_generate" => tool_test_generate(arguments),
        "specrail_test_set_status" => tool_test_set_status(arguments),
        "specrail_implement" => tool_implement(arguments),
        "specrail_verify" => tool_verify(arguments),
        "specrail_advance" => tool_advance(arguments),
        other => Ok(tool_error_payload(anyhow!("unknown tool '{other}'"))),
    }
}

#[derive(Serialize)]
struct WorkflowGuidance {
    stage: String,
    recommended_skill: String,
    summary: String,
    blockers: Vec<String>,
    next_tools: Vec<String>,
    active_feature_id: Option<String>,
    active_outcome_id: Option<String>,
    candidate_feature_id: Option<String>,
    candidate_outcome_id: Option<String>,
}

struct OutcomeWorkflowSnapshot {
    feature_id: String,
    outcome: OutcomeSpec,
    test_count: usize,
    planned_test_ids: Vec<String>,
}

#[derive(Serialize, Clone)]
struct OutcomeTestReview {
    feature_id: String,
    outcome_id: String,
    required_test_paths: Vec<String>,
    related_tests: Vec<TestSpec>,
    missing_required_tests: Vec<String>,
    planned_required_tests: Vec<String>,
    undeclared_tests: Vec<TestSpec>,
    suggested_test_paths: Vec<String>,
    openable_test_paths: Vec<String>,
    required_test_count: usize,
    related_test_count: usize,
    has_no_required_tests: bool,
    has_no_related_tests: bool,
    has_gaps: bool,
    needs_generation: bool,
}

fn build_outcome_test_review(outcome: &OutcomeSpec, manifest: &TestManifest) -> OutcomeTestReview {
    let required_test_paths = outcome.required_tests.clone();
    let related_tests: Vec<TestSpec> = manifest
        .tests
        .iter()
        .filter(|test| test.feature_id == outcome.feature_id && test.outcome_id == outcome.id)
        .cloned()
        .collect();

    let missing_required_tests: Vec<String> = required_test_paths
        .iter()
        .filter(|path| !related_tests.iter().any(|test| test.path == **path))
        .cloned()
        .collect();

    let planned_required_tests: Vec<String> = required_test_paths
        .iter()
        .filter(|path| {
            related_tests
                .iter()
                .any(|test| test.path == **path && test.status == TestStatus::Planned)
        })
        .cloned()
        .collect();

    let undeclared_tests: Vec<TestSpec> = related_tests
        .iter()
        .filter(|test| !required_test_paths.iter().any(|path| path == &test.path))
        .cloned()
        .collect();

    let mut suggested_test_paths = missing_required_tests.clone();
    for path in &planned_required_tests {
        if !suggested_test_paths.contains(path) {
            suggested_test_paths.push(path.clone());
        }
    }

    let openable_test_paths = related_tests
        .iter()
        .map(|test| test.path.clone())
        .collect();
    let has_no_required_tests = required_test_paths.is_empty();
    let has_no_related_tests = related_tests.is_empty();
    let has_gaps = has_no_required_tests
        || has_no_related_tests
        || !missing_required_tests.is_empty()
        || !planned_required_tests.is_empty();

    OutcomeTestReview {
        feature_id: outcome.feature_id.clone(),
        outcome_id: outcome.id.clone(),
        required_test_count: required_test_paths.len(),
        related_test_count: related_tests.len(),
        required_test_paths,
        related_tests,
        missing_required_tests,
        planned_required_tests,
        undeclared_tests,
        suggested_test_paths: suggested_test_paths.clone(),
        openable_test_paths,
        has_no_required_tests,
        has_no_related_tests,
        has_gaps,
        needs_generation: !suggested_test_paths.is_empty(),
    }
}

fn tool_status(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let repo = match Repository::discover(&cwd) {
        Ok(repo) => repo,
        Err(_) => {
            return Ok(tool_success_payload(
                format!("No specrail project found from {}.", cwd.display()),
                Some(json!({
                    "initialized": false,
                    "cwd": cwd.display().to_string(),
                    "workflow": WorkflowGuidance {
                        stage: "init".to_string(),
                        recommended_skill: "specrail-init".to_string(),
                        summary: "Initialize specrail before planning features, tests, or implementation.".to_string(),
                        blockers: vec!["No .specrail project was found from this working directory.".to_string()],
                        next_tools: vec!["specrail_init".to_string()],
                        active_feature_id: None,
                        active_outcome_id: None,
                        candidate_feature_id: None,
                        candidate_outcome_id: None,
                    }
                })),
            ));
        }
    };

    repo.ensure_hierarchy()?;
    let config = repo.load_config()?;
    let state = repo.load_state()?;
    let features = repo.list_features()?;
    let solutions = repo.list_solutions()?;
    let projects: Vec<_> = solutions
        .iter()
        .flat_map(|solution| repo.list_projects(&solution.id).unwrap_or_default())
        .collect();
    let components: Vec<_> = projects
        .iter()
        .flat_map(|project| repo.list_components(&project.id).unwrap_or_default())
        .collect();
    let manifest = repo.load_manifest()?;

    let passing = manifest
        .tests
        .iter()
        .filter(|test| test.status == TestStatus::Passing)
        .count();
    let written = manifest
        .tests
        .iter()
        .filter(|test| test.status == TestStatus::Written)
        .count();
    let planned = manifest
        .tests
        .iter()
        .filter(|test| test.status == TestStatus::Planned)
        .count();

    let feature_summaries: Vec<Value> = features
        .iter()
        .map(|feature| {
            let outcomes = repo.list_outcomes(&feature.id).unwrap_or_default();
            let verified = outcomes
                .iter()
                .filter(|outcome| {
                    outcome.status == crate::core::models::OutcomeStatus::Verified
                })
                .count();
            json!({
                "feature": feature,
                "outcomeCount": outcomes.len(),
                "verifiedOutcomeCount": verified
            })
        })
        .collect();

    let active_solution = state.active_solution.clone().unwrap_or_else(|| "(none)".to_string());
    let active_project = state.active_project.clone().unwrap_or_else(|| "(none)".to_string());
    let active_component = state.active_component.clone().unwrap_or_else(|| "(none)".to_string());
    let active_feature = state.active_feature.clone().unwrap_or_else(|| "(none)".to_string());
    let active_outcome = state.active_outcome.clone().unwrap_or_else(|| "(none)".to_string());
    let project_name = config.name.clone();
    let _feature_count = feature_summaries.len();
    let test_count = manifest.tests.len();
    let workflow = build_workflow_guidance(&repo, &state, &features, &manifest)?;

    Ok(tool_success_payload(
        {
            let active_feat_icon = if state.active_feature.is_some() { "⚡" } else { "○" };
            let active_out_icon = if state.active_outcome.is_some() { "⚡" } else { "○" };
            let mut lines = vec![
                format!("🚂 **{project_name}** — specrail project"),
                String::new(),
                format!("Active solution: {active_solution}"),
                format!("Active project : {active_project}"),
                format!("Active component: {active_component}"),
                format!("Active feature : {active_feat_icon} {active_feature}"),
                format!("Active outcome : {active_out_icon} {active_outcome}"),
                format!("Tests          : {test_count} total ({passing} passing, {written} written, {planned} planned)"),
                String::new(),
                format!("▶  {}", workflow.summary),
            ];
            if !workflow.blockers.is_empty() {
                lines.push(String::new());
                for b in &workflow.blockers {
                    lines.push(format!("  ⚠️  {b}"));
                }
            }
            lines.join("\n")
        },
        Some(json!({
            "initialized": true,
            "root": repo.root.display().to_string(),
            "config": config,
            "state": state,
            "solutions": solutions,
            "projects": projects,
            "components": components,
            "features": feature_summaries,
            "manifest": manifest,
            "testCounts": {
                "passing": passing,
                "written": written,
                "planned": planned
            },
            "workflow": workflow
        })),
    ))
}

fn tool_feature_list(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let features = repo.list_features()?;
    let count = features.len();
    let list: String = features
        .iter()
        .map(|f| format!("  • {} — {}", f.id, f.title))
        .collect::<Vec<_>>()
        .join("\n");
    let text = if list.is_empty() {
        "No features yet. Create one with specrail_feature_new.".to_string()
    } else {
        format!("Found {count} feature(s):\n{list}")
    };
    Ok(tool_success_payload(text, Some(json!({ "features": features }))))
}

fn tool_solution_list(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let solutions = repo.list_solutions()?;
    let count = solutions.len();
    let list = solutions
        .iter()
        .map(|solution| format!("  • {} — {}", solution.id, solution.title))
        .collect::<Vec<_>>()
        .join("\n");
    let text = if list.is_empty() {
        "No solutions yet. Create one with specrail_solution_new.".to_string()
    } else {
        format!("Found {count} solution(s):\n{list}")
    };
    Ok(tool_success_payload(text, Some(json!({ "solutions": solutions }))))
}

fn tool_solution_show(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let id = require_string(arguments, "id")?;
    let solution = repo.load_solution(&id)?;
    let projects = repo.list_projects(&id)?;
    Ok(tool_success_payload(
        format!("Loaded solution '{id}'."),
        Some(json!({ "solution": solution, "projects": projects })),
    ))
}

fn tool_project_list(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let solution_id = require_string(arguments, "solution_id")?;
    let projects = repo.list_projects(&solution_id)?;
    let count = projects.len();
    let list = projects
        .iter()
        .map(|project| format!("  • {} — {}", project.id, project.title))
        .collect::<Vec<_>>()
        .join("\n");
    let text = if list.is_empty() {
        format!("No projects found for solution '{solution_id}'.")
    } else {
        format!("Found {count} project(s) for '{solution_id}':\n{list}")
    };
    Ok(tool_success_payload(
        text,
        Some(json!({ "solution_id": solution_id, "projects": projects })),
    ))
}

fn tool_project_show(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let id = require_string(arguments, "id")?;
    let project = repo.load_project(&id)?;
    let components = repo.list_components(&id)?;
    Ok(tool_success_payload(
        format!("Loaded project '{id}'."),
        Some(json!({ "project": project, "components": components })),
    ))
}

fn tool_component_list(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let project_id = require_string(arguments, "project_id")?;
    let components = repo.list_components(&project_id)?;
    let count = components.len();
    let list = components
        .iter()
        .map(|component| format!("  • {} — {}", component.id, component.title))
        .collect::<Vec<_>>()
        .join("\n");
    let text = if list.is_empty() {
        format!("No components found for project '{project_id}'.")
    } else {
        format!("Found {count} component(s) for '{project_id}':\n{list}")
    };
    Ok(tool_success_payload(
        text,
        Some(json!({ "project_id": project_id, "components": components })),
    ))
}

fn tool_component_show(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let id = require_string(arguments, "id")?;
    let component = repo.load_component(&id)?;
    let features = repo.list_features_for_component(&id)?;
    Ok(tool_success_payload(
        format!("Loaded component '{id}'."),
        Some(json!({ "component": component, "features": features })),
    ))
}

fn feature_matches_active_hierarchy(
    feature: &FeatureSpec,
    active_solution_id: Option<&str>,
    active_project_id: Option<&str>,
    active_component_id: Option<&str>,
) -> bool {
    if let Some(component_id) = active_component_id {
        return feature.component_id == component_id;
    }

    if let Some(project_id) = active_project_id {
        return feature.project_id == project_id;
    }

    if let Some(solution_id) = active_solution_id {
        return feature.solution_id == solution_id;
    }

    true
}

fn feature_navigate_hierarchy_payload(
    active_solution_id: Option<String>,
    active_project_id: Option<String>,
    active_component_id: Option<String>,
    active_solution: Option<crate::core::models::SolutionSpec>,
    active_project: Option<crate::core::models::ProjectSpec>,
    active_component: Option<crate::core::models::ComponentSpec>,
) -> Value {
    json!({
        "activeSolutionId": active_solution_id,
        "activeProjectId": active_project_id,
        "activeComponentId": active_component_id,
        "activeSolution": active_solution,
        "activeProject": active_project,
        "activeComponent": active_component,
    })
}

fn feature_navigate_feature_actions() -> Value {
    json!({
        "solutionListTool": "specrail_solution_list",
        "projectListTool": "specrail_project_list",
        "componentListTool": "specrail_component_list",
        "activateSolutionTool": "specrail_solution_activate",
        "activateProjectTool": "specrail_project_activate",
        "activateComponentTool": "specrail_component_activate",
        "createSolutionTool": "specrail_solution_new",
        "createProjectTool": "specrail_project_new",
        "createComponentTool": "specrail_component_new",
        "selectFeatureTool": "specrail_feature_navigate",
        "createFeatureTool": "specrail_feature_new",
        "editFeatureTool": "specrail_feature_edit",
        "activateFeatureTool": "specrail_feature_activate",
        "statusTool": "specrail_status",
        "traceTool": "specrail_trace"
    })
}

fn feature_navigate_outcome_actions() -> Value {
    json!({
        "solutionListTool": "specrail_solution_list",
        "projectListTool": "specrail_project_list",
        "componentListTool": "specrail_component_list",
        "activateSolutionTool": "specrail_solution_activate",
        "activateProjectTool": "specrail_project_activate",
        "activateComponentTool": "specrail_component_activate",
        "selectOutcomeTool": "specrail_outcome_activate",
        "createOutcomeTool": "specrail_outcome_new",
        "editOutcomeTool": "specrail_outcome_edit",
        "activateOutcomeTool": "specrail_outcome_activate",
        "unverifyOutcomeTool": "specrail_outcome_unverify",
        "implementTool": "specrail_implement",
        "verifyTool": "specrail_verify",
        "advanceTool": "specrail_advance",
        "testListTool": "specrail_test_list",
        "testReviewTool": "specrail_outcome_test_review",
        "testSuggestTool": "specrail_test_suggest",
        "testGenerateTool": "specrail_test_generate",
        "featureShowTool": "specrail_feature_show",
        "outcomeShowTool": "specrail_outcome_show",
        "statusTool": "specrail_status",
        "traceTool": "specrail_trace"
    })
}

fn tool_feature_navigate(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    repo.ensure_hierarchy()?;
    let state = repo.load_state()?;
    let features = repo.list_features()?;
    let solutions = repo.list_solutions()?;
    let projects: Vec<_> = solutions
        .iter()
        .flat_map(|solution| repo.list_projects(&solution.id).unwrap_or_default())
        .collect();
    let components: Vec<_> = projects
        .iter()
        .flat_map(|project| repo.list_components(&project.id).unwrap_or_default())
        .collect();
    let manifest = repo.load_manifest()?;
    let active_solution_id = state.active_solution.clone();
    let active_project_id = state.active_project.clone();
    let active_component_id = state.active_component.clone();
    let scoped_feature_summaries: Vec<Value> = features
        .iter()
        .filter(|feature| {
            feature_matches_active_hierarchy(
                feature,
                active_solution_id.as_deref(),
                active_project_id.as_deref(),
                active_component_id.as_deref(),
            )
        })
        .map(|feature| {
            let outcomes = repo.list_outcomes(&feature.id).unwrap_or_default();
            let verified_outcome_count = outcomes
                .iter()
                .filter(|outcome| outcome.status == OutcomeStatus::Verified)
                .count();
            let active_outcome_count = outcomes
                .iter()
                .filter(|outcome| outcome.status == crate::core::models::OutcomeStatus::Active)
                .count();
            json!({
                "id": feature.id,
                "title": feature.title,
                "status": feature.status,
                "solutionId": feature.solution_id,
                "projectId": feature.project_id,
                "componentId": feature.component_id,
                "pathLabel": format!("{}/{}/{}", feature.solution_id, feature.project_id, feature.component_id),
                "outcomeCount": outcomes.len(),
                "verifiedOutcomeCount": verified_outcome_count,
                "activeOutcomeCount": active_outcome_count,
                "isActive": state.active_feature.as_deref() == Some(feature.id.as_str())
            })
        })
        .collect();

    let active_feature_id = state.active_feature.clone();
    let active_outcome_id = state.active_outcome.clone();

    let active_solution = active_solution_id
        .as_ref()
        .and_then(|id| solutions.iter().find(|solution| solution.id == *id))
        .cloned();
    let active_project = active_project_id
        .as_ref()
        .and_then(|id| projects.iter().find(|project| project.id == *id))
        .cloned();
    let active_component = active_component_id
        .as_ref()
        .and_then(|id| components.iter().find(|component| component.id == *id))
        .cloned();

    let visible_projects: Vec<_> = match active_solution_id.as_deref() {
        Some(solution_id) => projects
            .iter()
            .filter(|project| project.solution_id == solution_id)
            .cloned()
            .collect(),
        None => projects.clone(),
    };
    let visible_components: Vec<_> = match active_project_id.as_deref() {
        Some(project_id) => components
            .iter()
            .filter(|component| component.project_id == project_id)
            .cloned()
            .collect(),
        None => components.clone(),
    };
    let hierarchy_payload = feature_navigate_hierarchy_payload(
        active_solution_id.clone(),
        active_project_id.clone(),
        active_component_id.clone(),
        active_solution.clone(),
        active_project.clone(),
        active_component.clone(),
    );

    if let Some(feature_id) = optional_string(arguments, "feature_id") {
        let feature = repo.load_feature(&feature_id)?;
        let outcomes = repo.list_outcomes(&feature_id)?;
        let next_order = outcomes
            .iter()
            .map(|outcome| outcome.order)
            .max()
            .unwrap_or(0)
            + 1;
        let outcome_summaries: Vec<Value> = outcomes
            .iter()
            .map(|outcome| {
                let review = build_outcome_test_review(outcome, &manifest);
                let test_count = review.related_test_count;
                let planned_test_count = review.planned_required_tests.len();
                let passing_test_count = review
                    .related_tests
                    .iter()
                    .filter(|test| test.status == TestStatus::Passing)
                    .count();
                json!({
                    "id": outcome.id,
                    "title": outcome.title,
                    "goal": outcome.goal,
                    "status": outcome.status,
                    "order": outcome.order,
                    "isActive": active_outcome_id.as_deref() == Some(outcome.id.as_str()),
                    "testCount": test_count,
                    "plannedTestCount": planned_test_count,
                    "passingTestCount": passing_test_count,
                    "requiredTestCount": review.required_test_count,
                    "missingRequiredTestCount": review.missing_required_tests.len(),
                    "undeclaredTestCount": review.undeclared_tests.len(),
                    "hasTestGaps": review.has_gaps,
                    "needsTestGeneration": review.needs_generation,
                    "testReview": review,
                    "availableActions": outcome_available_actions(
                        &outcome.status,
                        active_outcome_id.as_deref() == Some(outcome.id.as_str()),
                        test_count,
                        review.needs_generation || review.has_no_required_tests,
                    )
                })
            })
            .collect();

        let mut payload = Map::new();
        payload.insert("mode".to_string(), json!("outcome_selection"));
        payload.insert("activeFeatureId".to_string(), json!(active_feature_id));
        payload.insert("activeOutcomeId".to_string(), json!(active_outcome_id));
        if let Value::Object(map) = hierarchy_payload.clone() {
            payload.extend(map);
        }
        payload.insert("selectedFeatureId".to_string(), json!(feature_id));
        payload.insert("selectedFeature".to_string(), json!(feature));
        payload.insert("solutions".to_string(), json!(solutions));
        payload.insert("projects".to_string(), json!(projects));
        payload.insert("components".to_string(), json!(components));
        payload.insert("visibleProjects".to_string(), json!(visible_projects));
        payload.insert("visibleComponents".to_string(), json!(visible_components));
        payload.insert("features".to_string(), json!(scoped_feature_summaries));
        payload.insert("outcomes".to_string(), json!(outcome_summaries));
        payload.insert("suggestedNewOutcomeOrder".to_string(), json!(next_order));
        payload.insert("nextActions".to_string(), feature_navigate_outcome_actions());

        return Ok(feature_navigate_payload(String::new(), Some(Value::Object(payload))));
    }

    let mut payload = Map::new();
    payload.insert("mode".to_string(), json!("feature_selection"));
    payload.insert("solutions".to_string(), json!(solutions));
    payload.insert("projects".to_string(), json!(projects));
    payload.insert("components".to_string(), json!(components));
    payload.insert("visibleProjects".to_string(), json!(visible_projects));
    payload.insert("visibleComponents".to_string(), json!(visible_components));
    if let Value::Object(map) = hierarchy_payload {
        payload.extend(map);
    }
    payload.insert("activeFeatureId".to_string(), json!(active_feature_id));
    payload.insert("activeOutcomeId".to_string(), json!(active_outcome_id));
    payload.insert("selectedFeatureId".to_string(), Value::Null);
    payload.insert("features".to_string(), json!(scoped_feature_summaries));
    payload.insert("nextActions".to_string(), feature_navigate_feature_actions());

    Ok(feature_navigate_payload(String::new(), Some(Value::Object(payload))))
}

fn outcome_available_actions(
    status: &OutcomeStatus,
    is_active: bool,
    test_count: usize,
    needs_test_generation: bool,
) -> Vec<&'static str> {
    let mut actions = vec!["edit", "show"];

    if test_count > 0 {
        actions.push("tests");
    }

    if needs_test_generation && !matches!(status, OutcomeStatus::Skipped) {
        actions.push("generate_tests");
    }

    if !matches!(status, OutcomeStatus::Verified | OutcomeStatus::Skipped) && !is_active {
        actions.push("activate");
    }

    if is_active {
        actions.push("implement");
        actions.push("verify");
    }

    if *status == OutcomeStatus::Verified && is_active {
        actions.push("advance");
    }

    if matches!(status, OutcomeStatus::Verified | OutcomeStatus::Failed | OutcomeStatus::Skipped) {
        actions.push("unverify");
    }

    actions
}

fn resource_definitions() -> Vec<Value> {
    vec![json!({
        "uri": FEATURE_NAVIGATE_APP_URI,
        "name": "specrail_feature_navigate",
        "title": "Specrail Feature Picker",
        "description": "Interactive feature and outcome picker for the specrail workflow.",
        "mimeType": UI_RESOURCE_MIME_TYPE
    })]
}

fn read_resource(params: Option<&Value>) -> Result<Value> {
    let uri = params
        .and_then(|value| value.get("uri"))
        .and_then(Value::as_str)
        .context("missing resource uri")?;

    match uri {
        FEATURE_NAVIGATE_APP_URI => Ok(json!({
            "contents": [
                {
                    "uri": FEATURE_NAVIGATE_APP_URI,
                    "mimeType": UI_RESOURCE_MIME_TYPE,
                    "text": FEATURE_NAVIGATE_APP_HTML,
                    "_meta": {
                        "ui": {
                            "prefersBorder": true
                        }
                    }
                }
            ]
        })),
        other => bail!("resource not found: {other}"),
    }
}

fn tool_feature_show(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let id = require_string(arguments, "id")?;
    let feature = repo.load_feature(&id)?;
    let outcomes = repo.list_outcomes(&id)?;
    Ok(tool_success_payload(
        format!("Loaded feature '{id}'."),
        Some(json!({
            "feature": feature,
            "outcomes": outcomes
        })),
    ))
}

fn tool_outcome_list(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    repo.load_feature(&feature_id)?;
    let outcomes = repo.list_outcomes(&feature_id)?;
    let count = outcomes.len();
    let list: String = outcomes
        .iter()
        .map(|o| format!("  {}. [{}] {} — {}", o.order, format!("{:?}", o.status).to_lowercase(), o.id, o.title))
        .collect::<Vec<_>>()
        .join("\n");
    let text = if list.is_empty() {
        format!("No outcomes found for feature '{feature_id}'.")
    } else {
        format!("Found {count} outcome(s) for '{feature_id}':\n{list}")
    };
    Ok(tool_success_payload(text, Some(json!({ "feature_id": feature_id, "outcomes": outcomes }))))
}

fn tool_outcome_show(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    let outcome = repo.load_outcome(&feature_id, &outcome_id)?;
    Ok(tool_success_payload(
        format!("Loaded outcome '{outcome_id}' for feature '{feature_id}'."),
        Some(json!({ "outcome": outcome })),
    ))
}

fn tool_outcome_test_review(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    let outcome = repo.load_outcome(&feature_id, &outcome_id)?;
    let manifest = repo.load_manifest()?;
    let review = build_outcome_test_review(&outcome, &manifest);

    Ok(tool_success_payload(
        format!(
            "Reviewed tests for outcome '{outcome_id}' in feature '{feature_id}'."
        ),
        Some(json!({ "outcome": outcome, "review": review })),
    ))
}

fn tool_test_list(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let manifest = repo.load_manifest()?;
    let feature_filter = optional_string(arguments, "feature_id");
    let outcome_filter = optional_string(arguments, "outcome_id");
    let tests: Vec<_> = manifest
        .tests
        .into_iter()
        .filter(|test| {
            feature_filter
                .as_deref()
                .map_or(true, |feature_id| test.feature_id == feature_id)
                && outcome_filter
                    .as_deref()
                    .map_or(true, |outcome_id| test.outcome_id == outcome_id)
        })
        .collect();

    let count = tests.len();
    Ok(tool_success_payload(
        format!("Found {count} test(s)."),
        Some(json!({ "tests": tests })),
    ))
}

fn tool_trace(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let limit = optional_usize(arguments, "limit")?;
    let events = Ledger::read_all(&repo.ledger_path())?;
    let events = match limit {
        Some(limit) => {
            let mut recent: Vec<_> = events.into_iter().rev().take(limit).collect();
            recent.reverse();
            recent
        }
        None => events,
    };
    let count = events.len();

    Ok(tool_success_payload(
        format!("Loaded {count} ledger event(s)."),
        Some(json!({ "events": events })),
    ))
}

fn tool_init(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let no_wizard = arguments
        .get("no_wizard")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let mut args = vec!["init".to_string()];
    if no_wizard {
        args.push("--no-wizard".to_string());
    }

    run_cli_tool(&cwd, args)
}

fn tool_solution_new(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    run_cli_tool(
        &cwd,
        vec![
            "solution".to_string(),
            "new".to_string(),
            id,
            "--title".to_string(),
            title,
            "--purpose".to_string(),
            purpose,
        ],
    )
}

fn tool_solution_activate(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    run_cli_tool(&cwd, vec!["solution".to_string(), "activate".to_string(), id])
}

fn tool_solution_edit(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    run_cli_tool(
        &cwd,
        vec![
            "solution".to_string(),
            "edit".to_string(),
            id,
            "--title".to_string(),
            title,
            "--purpose".to_string(),
            purpose,
        ],
    )
}

fn tool_project_new(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let solution_id = require_string(arguments, "solution_id")?;
    let id = require_string(arguments, "id")?;
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    run_cli_tool(
        &cwd,
        vec![
            "project".to_string(),
            "new".to_string(),
            solution_id,
            id,
            "--title".to_string(),
            title,
            "--purpose".to_string(),
            purpose,
        ],
    )
}

fn tool_project_activate(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    run_cli_tool(&cwd, vec!["project".to_string(), "activate".to_string(), id])
}

fn tool_project_edit(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    run_cli_tool(
        &cwd,
        vec![
            "project".to_string(),
            "edit".to_string(),
            id,
            "--title".to_string(),
            title,
            "--purpose".to_string(),
            purpose,
        ],
    )
}

fn tool_component_new(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let project_id = require_string(arguments, "project_id")?;
    let id = require_string(arguments, "id")?;
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    run_cli_tool(
        &cwd,
        vec![
            "component".to_string(),
            "new".to_string(),
            project_id,
            id,
            "--title".to_string(),
            title,
            "--purpose".to_string(),
            purpose,
        ],
    )
}

fn tool_component_activate(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    run_cli_tool(&cwd, vec!["component".to_string(), "activate".to_string(), id])
}

fn tool_component_edit(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    run_cli_tool(
        &cwd,
        vec![
            "component".to_string(),
            "edit".to_string(),
            id,
            "--title".to_string(),
            title,
            "--purpose".to_string(),
            purpose,
        ],
    )
}

fn tool_feature_new(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let component_id = optional_string(arguments, "component_id");
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    let outcomes = string_array(arguments, "outcomes")?;
    let constraints = string_array(arguments, "constraints")?;
    let non_goals = string_array(arguments, "non_goals")?;
    let dependencies = string_array(arguments, "dependencies")?;

    let mut args = vec![
        "feature".to_string(),
        "new".to_string(),
        id,
    ];

    if let Some(component_id) = component_id {
        args.push("--component".to_string());
        args.push(component_id);
    }

    args.push("--title".to_string());
    args.push(title);
    args.push("--purpose".to_string());
    args.push(purpose);

    push_repeated_flag(&mut args, "--outcome", outcomes);
    push_repeated_flag(&mut args, "--constraint", constraints);
    push_repeated_flag(&mut args, "--non-goal", non_goals);
    push_repeated_flag(&mut args, "--dep", dependencies);

    run_cli_tool(&cwd, args)
}

fn tool_feature_activate(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    run_cli_tool(&cwd, vec!["feature".to_string(), "activate".to_string(), id])
}

fn tool_feature_edit(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let title = require_string(arguments, "title")?;
    let purpose = require_string(arguments, "purpose")?;
    let outcomes = string_array(arguments, "outcomes")?;
    let constraints = string_array(arguments, "constraints")?;
    let non_goals = string_array(arguments, "non_goals")?;
    let dependencies = string_array(arguments, "dependencies")?;

    let mut args = vec![
        "feature".to_string(),
        "edit".to_string(),
        id,
        "--title".to_string(),
        title,
        "--purpose".to_string(),
        purpose,
    ];

    push_repeated_flag(&mut args, "--outcome", outcomes);
    push_repeated_flag(&mut args, "--constraint", constraints);
    push_repeated_flag(&mut args, "--non-goal", non_goals);
    push_repeated_flag(&mut args, "--dep", dependencies);

    run_cli_tool(&cwd, args)
}

fn tool_outcome_new(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    let title = require_string(arguments, "title")?;
    let goal = require_string(arguments, "goal")?;
    let order = require_u64(arguments, "order")?;
    let prerequisites = string_array(arguments, "prerequisites")?;
    let allowed_paths = string_array(arguments, "allowed_paths")?;
    let forbidden_paths = string_array(arguments, "forbidden_paths")?;
    let required_tests = string_array(arguments, "required_tests")?;

    let mut args = vec![
        "outcome".to_string(),
        "new".to_string(),
        feature_id,
        outcome_id,
        "--title".to_string(),
        title,
        "--goal".to_string(),
        goal,
        "--order".to_string(),
        order.to_string(),
    ];

    push_repeated_flag(&mut args, "--prereq", prerequisites);
    push_repeated_flag(&mut args, "--allow", allowed_paths);
    push_repeated_flag(&mut args, "--forbid", forbidden_paths);
    push_repeated_flag(&mut args, "--test", required_tests);

    run_cli_tool(&cwd, args)
}

fn tool_outcome_activate(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    run_cli_tool(
        &cwd,
        vec![
            "outcome".to_string(),
            "activate".to_string(),
            feature_id,
            outcome_id,
        ],
    )
}

fn tool_outcome_edit(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    let title = require_string(arguments, "title")?;
    let goal = require_string(arguments, "goal")?;
    let order = require_u64(arguments, "order")?;
    let prerequisites = string_array(arguments, "prerequisites")?;
    let allowed_paths = string_array(arguments, "allowed_paths")?;
    let forbidden_paths = string_array(arguments, "forbidden_paths")?;
    let required_tests = string_array(arguments, "required_tests")?;

    let mut args = vec![
        "outcome".to_string(),
        "edit".to_string(),
        feature_id,
        outcome_id,
        "--title".to_string(),
        title,
        "--goal".to_string(),
        goal,
        "--order".to_string(),
        order.to_string(),
    ];

    push_repeated_flag(&mut args, "--prereq", prerequisites);
    push_repeated_flag(&mut args, "--allow", allowed_paths);
    push_repeated_flag(&mut args, "--forbid", forbidden_paths);
    push_repeated_flag(&mut args, "--test", required_tests);

    run_cli_tool(&cwd, args)
}

fn tool_outcome_unverify(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    let outcome = crate::commands::outcome::reset_status_to_pending(&repo, &feature_id, &outcome_id)?;

    Ok(tool_success_payload(
        format!(
            "Outcome '{outcome_id}' for feature '{feature_id}' reset to pending."
        ),
        Some(json!({ "outcome": outcome })),
    ))
}

fn tool_outcome_add_required_test(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    let path = require_string(arguments, "path")?;
    let added = crate::commands::outcome::ensure_required_test(&repo, &feature_id, &outcome_id, &path)?;

    Ok(tool_success_payload(
        if added {
            format!(
                "Added '{}' to required_tests for outcome '{}:{}'.",
                path, feature_id, outcome_id
            )
        } else {
            format!(
                "'{}' is already listed in required_tests for outcome '{}:{}'.",
                path, feature_id, outcome_id
            )
        },
        Some(json!({
            "feature_id": feature_id,
            "outcome_id": outcome_id,
            "path": path,
            "added": added,
        })),
    ))
}

fn tool_test_add(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let feature_id = require_string(arguments, "feature_id")?;
    let outcome_id = require_string(arguments, "outcome_id")?;
    let path = require_string(arguments, "path")?;
    let kind = optional_string(arguments, "kind").unwrap_or_else(|| "unit".to_string());
    let purpose_refs = string_array(arguments, "purpose_refs")?;

    let mut args = vec![
        "test".to_string(),
        "add".to_string(),
        id,
        "--feature".to_string(),
        feature_id,
        "--outcome".to_string(),
        outcome_id,
        "--path".to_string(),
        path,
        "--kind".to_string(),
        kind,
    ];

    push_repeated_flag(&mut args, "--ref", purpose_refs);

    run_cli_tool(&cwd, args)
}

fn tool_test_generate(arguments: &Map<String, Value>) -> Result<Value> {
    let feature_id = optional_string(arguments, "feature_id");
    let outcome_id = optional_string(arguments, "outcome_id");
    let agent = optional_string(arguments, "agent");

    if feature_id.is_some() || outcome_id.is_some() {
        let repo = discover_repo(arguments)?;
        let summary = commands::test::generate_scoped(
            &repo,
            feature_id.as_deref(),
            outcome_id.as_deref(),
            agent.as_deref(),
        )?;

        return Ok(tool_success_payload(
            format!(
                "Generated {} test file(s) for {} with agent '{}'.",
                summary.generated_count, summary.scope_label, summary.agent
            ),
            Some(json!({ "generation": summary })),
        ));
    }

    let cwd = resolve_cwd(arguments)?;
    let mut args = vec!["test".to_string(), "generate".to_string()];
    if let Some(agent) = agent {
        args.push("--agent".to_string());
        args.push(agent);
    }
    run_cli_tool(&cwd, args)
}

fn tool_test_suggest(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let feature_id = optional_string(arguments, "feature_id");
    let outcome_id = optional_string(arguments, "outcome_id");
    let agent = optional_string(arguments, "agent");
    let preview = commands::test::preview(
        &repo,
        feature_id.as_deref(),
        outcome_id.as_deref(),
        agent.as_deref(),
    )?;

    Ok(tool_success_payload(
        format!(
            "Previewed {} suggested test file(s) with agent '{}' for {}.",
            preview.suggestions.len(),
            preview.agent,
            preview.scope_label
        ),
        Some(json!({ "preview": preview })),
    ))
}

fn tool_test_set_status(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    let status = require_string(arguments, "status")?;
    run_cli_tool(
        &cwd,
        vec!["test".to_string(), "set-status".to_string(), id, status],
    )
}

fn tool_implement(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let mut args = vec!["implement".to_string()];
    if let Some(agent) = optional_string(arguments, "agent") {
        args.push("--agent".to_string());
        args.push(agent);
    }
    run_cli_tool(&cwd, args)
}

fn tool_verify(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    run_cli_tool(&cwd, vec!["verify".to_string()])
}

fn tool_advance(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    run_cli_tool(&cwd, vec!["advance".to_string()])
}

fn run_cli_tool(cwd: &Path, args: Vec<String>) -> Result<Value> {
    let executable = std::env::current_exe().context("resolving specrail executable path")?;
    let output = Command::new(&executable)
        .args(&args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("running specrail {}", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let success = output.status.success();
    let exit_code = output.status.code();
    let command = format!("{} {}", executable.display(), args.join(" "));
    let text = build_cli_tool_text(success, exit_code, &stdout, &stderr);

    Ok(tool_payload(
        text,
        Some(json!(CliToolResult {
            command,
            cwd: cwd.display().to_string(),
            success,
            exit_code,
            stdout,
            stderr,
        })),
        !success,
    ))
}

fn build_cli_tool_text(
    success: bool,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
) -> String {
    let mut parts = Vec::new();

    if !stdout.trim().is_empty() {
        parts.push(stdout.trim().to_string());
    }
    if !stderr.trim().is_empty() {
        parts.push(format!("stderr:\n{}", stderr.trim()));
    }

    if parts.is_empty() {
        parts.push(if success {
            "Command completed successfully.".to_string()
        } else {
            format!("Command failed with exit code {}.", exit_code.unwrap_or(-1))
        });
    }

    parts.join("\n\n")
}

fn build_workflow_guidance(
    repo: &Repository,
    state: &ProjectState,
    features: &[FeatureSpec],
    manifest: &TestManifest,
) -> Result<WorkflowGuidance> {
    if features.is_empty() {
        return Ok(WorkflowGuidance {
            stage: "workflow".to_string(),
            recommended_skill: "specrail-workflow".to_string(),
            summary: "No features exist yet. Gather the first feature and break it into ordered outcomes before planning tests.".to_string(),
            blockers: vec!["The project has been initialized, but no features are registered yet.".to_string()],
            next_tools: vec!["specrail_feature_new".to_string(), "specrail_outcome_new".to_string()],
            active_feature_id: state.active_feature.clone(),
            active_outcome_id: state.active_outcome.clone(),
            candidate_feature_id: None,
            candidate_outcome_id: None,
        });
    }

    let mut any_outcomes = false;
    for feature in features {
        if !repo.list_outcomes(&feature.id)?.is_empty() {
            any_outcomes = true;
            break;
        }
    }

    if !any_outcomes {
        return Ok(WorkflowGuidance {
            stage: "workflow".to_string(),
            recommended_skill: "specrail-workflow".to_string(),
            summary: "Features exist, but no outcomes are defined yet. Split the next feature into outcome-sized slices before planning tests.".to_string(),
            blockers: vec!["At least one feature is present, but there are no outcomes to drive the TDD loop yet.".to_string()],
            next_tools: vec!["specrail_outcome_new".to_string()],
            active_feature_id: state.active_feature.clone(),
            active_outcome_id: state.active_outcome.clone(),
            candidate_feature_id: state.active_feature.clone(),
            candidate_outcome_id: None,
        });
    }

    if let (Some(feature_id), Some(outcome_id)) =
        (state.active_feature.as_deref(), state.active_outcome.as_deref())
    {
        let active = workflow_snapshot(repo, manifest, feature_id, outcome_id)?;
        return workflow_guidance_for_snapshot(
            repo,
            state,
            Some(&active),
            find_first_incomplete_outcome(repo, features, manifest)?,
        );
    }

    let candidate = find_first_incomplete_outcome(repo, features, manifest)?;
    workflow_guidance_for_snapshot(repo, state, None, candidate)
}

fn workflow_guidance_for_snapshot(
    repo: &Repository,
    state: &ProjectState,
    active: Option<&OutcomeWorkflowSnapshot>,
    candidate: Option<OutcomeWorkflowSnapshot>,
) -> Result<WorkflowGuidance> {
    if let Some(active) = active {
        let next_tools = if active.outcome.status == OutcomeStatus::Verified {
            vec!["specrail_advance".to_string()]
        } else if active.outcome.status == OutcomeStatus::Failed {
            vec!["specrail_implement".to_string(), "specrail_verify".to_string()]
        } else if active.test_count == 0 || !active.planned_test_ids.is_empty() {
            vec![
                "specrail_test_add".to_string(),
                "specrail_test_generate".to_string(),
                "specrail_test_set_status".to_string(),
            ]
        } else {
            vec!["specrail_implement".to_string(), "specrail_verify".to_string()]
        };

        let blockers = if active.test_count == 0 {
            vec![format!(
                "Active outcome '{}:{}' has no registered tests yet.",
                active.feature_id, active.outcome.id
            )]
        } else if !active.planned_test_ids.is_empty() {
            vec![format!(
                "Active outcome '{}:{}' still has tests in planned status: {}.",
                active.feature_id,
                active.outcome.id,
                active.planned_test_ids.join(", ")
            )]
        } else if active.outcome.status == OutcomeStatus::Failed {
            vec![format!(
                "Active outcome '{}:{}' failed verification and must be fixed before advancing.",
                active.feature_id, active.outcome.id
            )]
        } else {
            Vec::new()
        };

        let summary = match active.outcome.status {
            OutcomeStatus::Verified => {
                let outcomes = repo.list_outcomes(&active.feature_id)?;
                let has_next = outcomes.iter().any(|outcome| outcome.order == active.outcome.order + 1);
                if has_next {
                    format!(
                        "Active outcome '{}:{}' is verified. Advance to unlock the next outcome.",
                        active.feature_id, active.outcome.id
                    )
                } else {
                    format!(
                        "Active outcome '{}:{}' is verified. Advance once more to mark the feature complete.",
                        active.feature_id, active.outcome.id
                    )
                }
            }
            OutcomeStatus::Failed => format!(
                "Active outcome '{}:{}' failed verification. Keep it active, fix the implementation, then verify again.",
                active.feature_id, active.outcome.id
            ),
            _ if active.test_count == 0 => format!(
                "Active outcome '{}:{}' is blocked on test planning. Register at least one test before implementation.",
                active.feature_id, active.outcome.id
            ),
            _ if !active.planned_test_ids.is_empty() => format!(
                "Active outcome '{}:{}' is blocked on test authoring. Move planned tests to written before implementation.",
                active.feature_id, active.outcome.id
            ),
            _ => format!(
                "Active outcome '{}:{}' is ready for the implement → verify loop.",
                active.feature_id, active.outcome.id
            ),
        };

        return Ok(WorkflowGuidance {
            stage: workflow_stage(active),
            recommended_skill: workflow_skill(active).to_string(),
            summary,
            blockers,
            next_tools,
            active_feature_id: Some(active.feature_id.clone()),
            active_outcome_id: Some(active.outcome.id.clone()),
            candidate_feature_id: Some(active.feature_id.clone()),
            candidate_outcome_id: Some(active.outcome.id.clone()),
        });
    }

    let Some(candidate) = candidate else {
        return Ok(WorkflowGuidance {
            stage: "done".to_string(),
            recommended_skill: "specrail-activation".to_string(),
            summary: "All known outcomes are already verified or skipped. The TDD workflow is complete.".to_string(),
            blockers: Vec::new(),
            next_tools: vec!["specrail_status".to_string(), "specrail_trace".to_string()],
            active_feature_id: state.active_feature.clone(),
            active_outcome_id: state.active_outcome.clone(),
            candidate_feature_id: None,
            candidate_outcome_id: None,
        });
    };

    let (summary, blockers, next_tools, stage, recommended_skill) = if candidate.test_count == 0 {
        (
            format!(
                "Next outcome '{}:{}' is blocked on test planning. Register at least one test before activation.",
                candidate.feature_id, candidate.outcome.id
            ),
            vec![format!(
                "Outcome '{}:{}' has no registered tests yet.",
                candidate.feature_id, candidate.outcome.id
            )],
            vec![
                "specrail_test_add".to_string(),
                "specrail_test_generate".to_string(),
                "specrail_test_set_status".to_string(),
            ],
            "testing".to_string(),
            "specrail-testing".to_string(),
        )
    } else if !candidate.planned_test_ids.is_empty() {
        (
            format!(
                "Next outcome '{}:{}' is blocked on test authoring. Move planned tests to written before implementation.",
                candidate.feature_id, candidate.outcome.id
            ),
            vec![format!(
                "Outcome '{}:{}' still has tests in planned status: {}.",
                candidate.feature_id,
                candidate.outcome.id,
                candidate.planned_test_ids.join(", ")
            )],
            vec![
                "specrail_test_generate".to_string(),
                "specrail_test_set_status".to_string(),
            ],
            "testing".to_string(),
            "specrail-testing".to_string(),
        )
    } else {
        (
            format!(
                "Next outcome '{}:{}' is ready to activate and run through implement → verify → advance.",
                candidate.feature_id, candidate.outcome.id
            ),
            Vec::new(),
            vec![
                "specrail_feature_activate".to_string(),
                "specrail_outcome_activate".to_string(),
                "specrail_implement".to_string(),
                "specrail_verify".to_string(),
                "specrail_advance".to_string(),
            ],
            "activation".to_string(),
            "specrail-activation".to_string(),
        )
    };

    Ok(WorkflowGuidance {
        stage,
        recommended_skill,
        summary,
        blockers,
        next_tools,
        active_feature_id: state.active_feature.clone(),
        active_outcome_id: state.active_outcome.clone(),
        candidate_feature_id: Some(candidate.feature_id),
        candidate_outcome_id: Some(candidate.outcome.id),
    })
}

fn workflow_stage(snapshot: &OutcomeWorkflowSnapshot) -> String {
    if snapshot.outcome.status != OutcomeStatus::Failed
        && snapshot.outcome.status != OutcomeStatus::Verified
        && (snapshot.test_count == 0 || !snapshot.planned_test_ids.is_empty())
    {
        "testing".to_string()
    } else {
        "activation".to_string()
    }
}

fn workflow_skill(snapshot: &OutcomeWorkflowSnapshot) -> &'static str {
    if snapshot.outcome.status == OutcomeStatus::Verified
        || snapshot.outcome.status == OutcomeStatus::Failed
    {
        "specrail-activation"
    } else if snapshot.test_count == 0 || !snapshot.planned_test_ids.is_empty() {
        "specrail-testing"
    } else {
        "specrail-activation"
    }
}

fn find_first_incomplete_outcome(
    repo: &Repository,
    features: &[FeatureSpec],
    manifest: &TestManifest,
) -> Result<Option<OutcomeWorkflowSnapshot>> {
    for feature in features {
        for outcome in repo.list_outcomes(&feature.id)? {
            if outcome.status != OutcomeStatus::Verified && outcome.status != OutcomeStatus::Skipped {
                return workflow_snapshot(repo, manifest, &feature.id, &outcome.id).map(Some);
            }
        }
    }

    Ok(None)
}

fn workflow_snapshot(
    repo: &Repository,
    manifest: &TestManifest,
    feature_id: &str,
    outcome_id: &str,
) -> Result<OutcomeWorkflowSnapshot> {
    let outcome = repo.load_outcome(feature_id, outcome_id)?;
    let tests: Vec<_> = manifest
        .tests
        .iter()
        .filter(|test| test.feature_id == feature_id && test.outcome_id == outcome_id)
        .collect();
    let planned_test_ids = tests
        .iter()
        .filter(|test| test.status == TestStatus::Planned)
        .map(|test| test.id.clone())
        .collect();

    Ok(OutcomeWorkflowSnapshot {
        feature_id: feature_id.to_string(),
        outcome,
        test_count: tests.len(),
        planned_test_ids,
    })
}

fn discover_repo(arguments: &Map<String, Value>) -> Result<Repository> {
    let cwd = resolve_cwd(arguments)?;
    let repo = Repository::discover(&cwd)?;
    repo.ensure_hierarchy()?;
    Ok(repo)
}

fn resolve_cwd(arguments: &Map<String, Value>) -> Result<PathBuf> {
    match arguments.get("cwd") {
        Some(Value::String(path)) => Ok(PathBuf::from(path)),
        Some(_) => bail!("cwd must be a string path"),
        None => std::env::current_dir().context("resolving current working directory"),
    }
}

fn require_string(arguments: &Map<String, Value>, key: &str) -> Result<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("missing string argument '{key}'"))
}

fn optional_string(arguments: &Map<String, Value>, key: &str) -> Option<String> {
    arguments.get(key).and_then(Value::as_str).map(str::to_string)
}

fn require_u64(arguments: &Map<String, Value>, key: &str) -> Result<u64> {
    arguments
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("missing integer argument '{key}'"))
}

fn optional_usize(arguments: &Map<String, Value>, key: &str) -> Result<Option<usize>> {
    arguments
        .get(key)
        .map(|value| {
            value
                .as_u64()
                .map(|raw| raw as usize)
                .ok_or_else(|| anyhow!("argument '{key}' must be an integer"))
        })
        .transpose()
}

fn string_array(arguments: &Map<String, Value>, key: &str) -> Result<Vec<String>> {
    match arguments.get(key) {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| anyhow!("argument '{key}' must only contain strings"))
            })
            .collect(),
        Some(_) => bail!("argument '{key}' must be an array of strings"),
        None => Ok(Vec::new()),
    }
}

fn push_repeated_flag(args: &mut Vec<String>, flag: &str, values: Vec<String>) {
    for value in values {
        args.push(flag.to_string());
        args.push(value);
    }
}

fn tool_success_payload(text: String, structured_content: Option<Value>) -> Value {
    tool_payload(text, structured_content, false)
}

fn feature_navigate_payload(text: String, structured_content: Option<Value>) -> Value {
    tool_payload_with_meta(
        text,
        structured_content,
        Some(json!({
            "ui": {
                "resourceUri": FEATURE_NAVIGATE_APP_URI,
                "visibility": ["model", "app"]
            },
            "ui/resourceUri": FEATURE_NAVIGATE_APP_URI
        })),
        false,
    )
}

fn tool_error_payload(error: impl std::fmt::Display) -> Value {
    tool_payload(error.to_string(), None, true)
}

fn tool_payload(text: String, structured_content: Option<Value>, is_error: bool) -> Value {
    tool_payload_with_meta(text, structured_content, None, is_error)
}

fn tool_payload_with_meta(
    text: String,
    structured_content: Option<Value>,
    meta: Option<Value>,
    is_error: bool,
) -> Value {
    let mut result = json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ],
        "isError": is_error
    });

    if let Some(structured_content) = structured_content {
        result["structuredContent"] = structured_content;
    }

    if let Some(meta) = meta {
        result["_meta"] = meta;
    }

    result
}

#[derive(Serialize)]
struct CliToolResult {
    command: String,
    cwd: String,
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "specrail_status",
            "title": "Project Status",
            "description": "Get the current specrail project status, active workflow state, and recommended next action. Always call this first to understand where you are in the TDD workflow. Returns structuredContent.workflow with recommended_skill, blockers, next_tools, and candidate feature/outcome.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Workspace or project root path. Defaults to the server's working directory." }
                }
            }
        }),
        json!({
            "name": "specrail_feature_navigate",
            "title": "Workflow Navigator",
            "description": "Interactive solution/project/component/feature/outcome browser. Without feature_id returns feature cards annotated with their solution/project/component path. With feature_id returns the selected feature's outcomes with test counts and status.",
            "_meta": {
                "ui": {
                    "resourceUri": FEATURE_NAVIGATE_APP_URI,
                    "visibility": ["model", "app"]
                }
            },
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Workspace or project root path." },
                    "feature_id": { "type": "string", "description": "Feature identifier to drill into outcomes for." }
                }
            }
        }),
        json!({
            "name": "specrail_solution_list",
            "title": "List Solutions",
            "description": "List all solutions registered in the current specrail project.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "specrail_solution_show",
            "title": "Show Solution",
            "description": "Show a solution and the projects it contains.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_project_list",
            "title": "List Projects",
            "description": "List all projects for a solution.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "solution_id": { "type": "string" }
                },
                "required": ["solution_id"]
            }
        }),
        json!({
            "name": "specrail_project_show",
            "title": "Show Project",
            "description": "Show a project and the components it contains.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_component_list",
            "title": "List Components",
            "description": "List all components for a project.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "project_id": { "type": "string" }
                },
                "required": ["project_id"]
            }
        }),
        json!({
            "name": "specrail_component_show",
            "title": "Show Component",
            "description": "Show a component and the features it contains.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_feature_list",
            "title": "List Features",
            "description": "List all features registered in the current specrail project with their status and hierarchy path.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Workspace or project root path." }
                }
            }
        }),
        json!({
            "name": "specrail_feature_show",
            "title": "Show Feature",
            "description": "Show full details of a feature including its title, purpose, constraints, non-goals, and list of outcomes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string", "description": "Feature identifier (e.g. 'auth', 'payment')." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_outcome_list",
            "title": "List Outcomes",
            "description": "List all outcomes for a feature, ordered by their execution order. Each outcome has an id, title, status, and order number.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Feature identifier whose outcomes to list." }
                },
                "required": ["feature_id"]
            }
        }),
        json!({
            "name": "specrail_outcome_show",
            "title": "Show Outcome",
            "description": "Show full details of a single outcome including title, goal, status, order, prerequisites, allowed/forbidden paths, and required tests.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Feature identifier." },
                    "outcome_id": { "type": "string", "description": "Outcome identifier." }
                },
                "required": ["feature_id", "outcome_id"]
            }
        }),
        json!({
            "name": "specrail_outcome_test_review",
            "title": "Review Outcome Tests",
            "description": "Review one outcome's test health. Returns related manifest tests, missing required_tests paths, planned required tests, undeclared tests, and suggested test paths to add or generate.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Feature identifier." },
                    "outcome_id": { "type": "string", "description": "Outcome identifier." }
                },
                "required": ["feature_id", "outcome_id"]
            }
        }),
        json!({
            "name": "specrail_test_list",
            "title": "List Tests",
            "description": "List tests from the specrail manifest. Filter by feature_id and/or outcome_id. Returns tests with their status (planned, written, passing, failing). Use to check whether tests are ready before implementation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Filter to tests for this feature." },
                    "outcome_id": { "type": "string", "description": "Filter to tests for this outcome (requires feature_id)." }
                }
            }
        }),
        json!({
            "name": "specrail_test_suggest",
            "title": "Preview Test Suggestions",
            "description": "Preview agent-generated required test suggestions without writing files or changing the manifest. Can be scoped to one feature and outcome to inspect missing test ideas safely before generation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Optional feature identifier to scope preview." },
                    "outcome_id": { "type": "string", "description": "Optional outcome identifier within feature_id." },
                    "agent": { "type": "string", "description": "Optional agent override for previewing suggestions." }
                }
            }
        }),
        json!({
            "name": "specrail_trace",
            "title": "Audit Ledger",
            "description": "Read the append-only specrail audit ledger showing the history of all project actions (features created, outcomes activated, tests updated, etc). Use limit to get recent events.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "description": "Max number of most recent events to return." }
                }
            }
        }),
        json!({
            "name": "specrail_init",
            "title": "Initialize Project",
            "description": "Initialize a specrail project in the given directory. Creates the .specrail/ directory structure with project.yaml, default solution/project/component hierarchy, test manifest, and state files. Safe to run on an existing project (only adds missing files).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Directory to initialize. Defaults to the server working directory." },
                    "no_wizard": { "type": "boolean", "description": "Skip the interactive setup wizard. Defaults to true for MCP usage." }
                }
            }
        }),
        json!({
            "name": "specrail_solution_new",
            "title": "New Solution",
            "description": "Create a new solution above projects, components, features, and outcomes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "purpose": { "type": "string" }
                },
                "required": ["id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_solution_activate",
            "title": "Activate Solution",
            "description": "Set a solution as the active top-level focus and clear deeper selections.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_solution_edit",
            "title": "Edit Solution",
            "description": "Update an existing solution's title and purpose.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "purpose": { "type": "string" }
                },
                "required": ["id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_project_new",
            "title": "New Project",
            "description": "Create a new project inside a solution.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "solution_id": { "type": "string" },
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "purpose": { "type": "string" }
                },
                "required": ["solution_id", "id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_project_activate",
            "title": "Activate Project",
            "description": "Set a project as the active focus and clear deeper selections.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_project_edit",
            "title": "Edit Project",
            "description": "Update an existing project's title and purpose.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "purpose": { "type": "string" }
                },
                "required": ["id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_component_new",
            "title": "New Component",
            "description": "Create a new component inside a project.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "project_id": { "type": "string" },
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "purpose": { "type": "string" }
                },
                "required": ["project_id", "id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_component_activate",
            "title": "Activate Component",
            "description": "Set a component as the active focus and clear deeper selections.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_component_edit",
            "title": "Edit Component",
            "description": "Update an existing component's title and purpose.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "purpose": { "type": "string" }
                },
                "required": ["id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_feature_new",
            "title": "New Feature",
            "description": "Create a new feature in the specrail project. A feature belongs to a component (which belongs to a project and solution) and groups ordered outcomes that together deliver a capability.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string", "description": "Unique slug identifier (e.g. 'auth', 'checkout')." },
                    "component_id": { "type": "string", "description": "Optional component identifier. Defaults to the active/default component." },
                    "title": { "type": "string", "description": "Human-readable feature title." },
                    "purpose": { "type": "string", "description": "One or two sentence description of why this feature is needed." },
                    "outcomes": { "type": "array", "items": { "type": "string" }, "description": "High-level outcome descriptions (optional, for documentation)." },
                    "constraints": { "type": "array", "items": { "type": "string" }, "description": "Technical or business constraints." },
                    "non_goals": { "type": "array", "items": { "type": "string" }, "description": "Explicitly out-of-scope items." },
                    "dependencies": { "type": "array", "items": { "type": "string" }, "description": "Feature IDs this feature depends on." }
                },
                "required": ["id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_feature_activate",
            "title": "Activate Feature",
            "description": "Set a feature as the active focus for the TDD workflow. Only one feature can be active at a time. Call this before activating an outcome to work on a feature's next outcome.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string", "description": "Feature identifier to activate." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_feature_edit",
            "title": "Edit Feature",
            "description": "Update an existing feature's title, purpose, outcomes list, constraints, non-goals, or dependencies. All fields must be supplied (include unchanged values).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string", "description": "Feature identifier to edit." },
                    "title": { "type": "string" },
                    "purpose": { "type": "string" },
                    "outcomes": { "type": "array", "items": { "type": "string" } },
                    "constraints": { "type": "array", "items": { "type": "string" } },
                    "non_goals": { "type": "array", "items": { "type": "string" } },
                    "dependencies": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["id", "title", "purpose"]
            }
        }),
        json!({
            "name": "specrail_outcome_new",
            "title": "New Outcome",
            "description": "Create a new outcome for a feature. Outcomes are the step-by-step milestones that drive the TDD loop. Assign an order number (1, 2, 3…) so outcomes are activated in sequence. After creating, call specrail_outcome_activate.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Parent feature identifier." },
                    "outcome_id": { "type": "string", "description": "Unique slug identifier within the feature (e.g. 'login', 'register')." },
                    "title": { "type": "string", "description": "Short outcome title." },
                    "goal": { "type": "string", "description": "One sentence description of what this outcome proves when verified." },
                    "order": { "type": "integer", "minimum": 1, "description": "Execution order (1 = first)." },
                    "prerequisites": { "type": "array", "items": { "type": "string" }, "description": "Outcome IDs that must be verified first." },
                    "allowed_paths": { "type": "array", "items": { "type": "string" }, "description": "Glob patterns the agent is allowed to modify." },
                    "forbidden_paths": { "type": "array", "items": { "type": "string" }, "description": "Glob patterns the agent must not modify." },
                    "required_tests": { "type": "array", "items": { "type": "string" }, "description": "Test IDs or paths that must pass." }
                },
                "required": ["feature_id", "outcome_id", "title", "goal", "order"]
            }
        }),
        json!({
            "name": "specrail_outcome_activate",
            "title": "Activate Outcome",
            "description": "Set an outcome as the active target for the implement → verify → advance TDD loop. Also sets the parent feature as active. Call this before specrail_implement. Cannot activate an already-verified outcome (use specrail_advance instead).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Feature identifier." },
                    "outcome_id": { "type": "string", "description": "Outcome identifier to activate." }
                },
                "required": ["feature_id", "outcome_id"]
            }
        }),
        json!({
            "name": "specrail_outcome_edit",
            "title": "Edit Outcome",
            "description": "Update an existing outcome's title, goal, order, prerequisites, allowed/forbidden paths, or required tests. Editing a verified/failed/skipped outcome resets it to pending. All fields must be supplied.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string" },
                    "outcome_id": { "type": "string" },
                    "title": { "type": "string" },
                    "goal": { "type": "string" },
                    "order": { "type": "integer", "minimum": 1 },
                    "prerequisites": { "type": "array", "items": { "type": "string" } },
                    "allowed_paths": { "type": "array", "items": { "type": "string" } },
                    "forbidden_paths": { "type": "array", "items": { "type": "string" } },
                    "required_tests": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["feature_id", "outcome_id", "title", "goal", "order"]
            }
        }),
        json!({
            "name": "specrail_outcome_unverify",
            "title": "Reset Outcome To Pending",
            "description": "Reset a verified, failed, or skipped outcome back to pending without changing its title, goal, or path constraints. Use this when you want to reopen an outcome for more implementation work.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Feature identifier." },
                    "outcome_id": { "type": "string", "description": "Outcome identifier to reset." }
                },
                "required": ["feature_id", "outcome_id"]
            }
        }),
        json!({
            "name": "specrail_outcome_add_required_test",
            "title": "Add Required Test",
            "description": "Add an existing related test path into the outcome's required_tests list so the outcome YAML and manifest stay aligned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string" },
                    "outcome_id": { "type": "string" },
                    "path": { "type": "string", "description": "Related test path to add into required_tests." }
                },
                "required": ["feature_id", "outcome_id", "path"]
            }
        }),
        json!({
            "name": "specrail_test_add",
            "title": "Add Test",
            "description": "Register a test in the specrail manifest for a specific outcome. Tests start in 'planned' status. Change to 'written' with specrail_test_set_status once the test file exists. At least one written (non-planned) test is required before specrail_implement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string", "description": "Unique test identifier." },
                    "feature_id": { "type": "string", "description": "Feature this test belongs to." },
                    "outcome_id": { "type": "string", "description": "Outcome this test validates." },
                    "path": { "type": "string", "description": "Relative file path of the test (e.g. 'tests/auth_login_test.rs')." },
                    "kind": { "type": "string", "enum": ["unit", "integration", "e2e"], "description": "Test kind. Defaults to 'unit'." },
                    "purpose_refs": { "type": "array", "items": { "type": "string" }, "description": "Outcome IDs or acceptance criteria references this test covers." }
                },
                "required": ["id", "feature_id", "outcome_id", "path"]
            }
        }),
        json!({
            "name": "specrail_test_generate",
            "title": "Generate Tests",
            "description": "Use the configured AI agent to generate test files for planned or missing required tests. For a scoped outcome with no required_tests yet, the agent can bootstrap an initial test path and specrail will persist it back into the outcome YAML.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string", "description": "Optional feature ID to scope generation to a specific outcome." },
                    "outcome_id": { "type": "string", "description": "Optional outcome ID to scope generation to a specific outcome. Requires feature_id." },
                    "agent": { "type": "string", "description": "Override the configured agent (e.g. 'generic-shell', 'copilot', 'codex')." }
                }
            }
        }),
        json!({
            "name": "specrail_test_set_status",
            "title": "Update Test Status",
            "description": "Update a test's status in the manifest. Allowed transitions: planned → written (test file written), written → passing (test now passes), written/passing → failing (regression). All tests must be non-planned before specrail_implement can run.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string", "description": "Test identifier to update." },
                    "status": { "type": "string", "enum": ["planned", "written", "passing", "failing"], "description": "New status for the test." }
                },
                "required": ["id", "status"]
            }
        }),
        json!({
            "name": "specrail_implement",
            "title": "Implement",
            "description": "Run the AI implementation agent for the active outcome. Requires: active feature, active outcome, at least one registered test, and no tests in 'planned' status. The agent receives a structured prompt and the list of test paths to pass. After implementing, run specrail_verify.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "agent": { "type": "string", "description": "Override the configured agent (e.g. 'generic-shell', 'copilot', 'codex')." }
                }
            }
        }),
        json!({
            "name": "specrail_verify",
            "title": "Verify",
            "description": "Run the project's test command (from project.yaml) to verify the active outcome. Marks the outcome 'verified' on success or 'failed' on failure. After a failed verify, fix the implementation and call verify again. After verified, call specrail_advance.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "specrail_advance",
            "title": "Advance",
            "description": "Advance from a verified outcome to the next one in sequence (by order + 1). If no next outcome exists, marks the feature complete. Requires the current active outcome to be in 'verified' status.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" }
                }
            }
        }),
    ]
}

fn read_message(reader: &mut impl BufRead) -> Result<Option<(Value, MessageTransport)>> {
    let mut content_length = None;
    mcp_debug_log("read_message: waiting for headers");

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            mcp_debug_log("read_message: EOF");
            return Ok(None);
        }

        let line = line.trim_end_matches(['\r', '\n']);
        mcp_debug_log(format!("read_message header: {line:?}"));

        if content_length.is_none() && line.starts_with('{') {
            mcp_debug_log("read_message: treating first line as raw JSON payload");
            let message = serde_json::from_str(line).context("parsing raw JSON-RPC payload")?;
            return Ok(Some((message, MessageTransport::RawJsonLine)));
        }

        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                let length = value
                    .trim()
                    .parse::<usize>()
                    .context("parsing Content-Length header")?;
                content_length = Some(length);
            }
        }
    }

    let content_length = content_length.context("missing Content-Length header")?;
    mcp_debug_log(format!("read_message: content_length={content_length}"));
    let mut payload = vec![0; content_length];
    reader.read_exact(&mut payload)?;
    mcp_debug_log(format!(
        "read_message payload: {}",
        summarize_for_log(&String::from_utf8_lossy(&payload))
    ));
    let message = serde_json::from_slice(&payload).context("parsing JSON-RPC payload")?;
    Ok(Some((message, MessageTransport::ContentLength)))
}

fn write_message(
    writer: &mut impl Write,
    message: &Value,
    transport: MessageTransport,
) -> Result<()> {
    let payload = serde_json::to_vec(message)?;
    match transport {
        MessageTransport::ContentLength => {
            write!(writer, "Content-Length: {}\r\n\r\n", payload.len())?;
            writer.write_all(&payload)?;
        }
        MessageTransport::RawJsonLine => {
            writer.write_all(&payload)?;
            writer.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn initialize_returns_tools_capability_details() {
        let mut server = McpServer::default();
        let response = server
            .handle_message(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": DEFAULT_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "specrail-test-client",
                        "version": "0.1.0"
                    }
                }
            }))
            .expect("initialize should return a response");

        assert_eq!(response["result"]["protocolVersion"], DEFAULT_PROTOCOL_VERSION);
        assert_eq!(response["result"]["capabilities"]["tools"]["listChanged"], false);
        assert_eq!(response["result"]["serverInfo"]["name"], "specrail");
        assert_eq!(server.protocol_version, DEFAULT_PROTOCOL_VERSION);
    }

    #[test]
    fn initialize_negotiates_back_to_supported_protocol_version() {
        let mut server = McpServer::default();
        let response = server
            .handle_message(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2026-01-01",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "specrail-test-client",
                        "version": "0.1.0"
                    }
                }
            }))
            .expect("initialize should return a response");

        assert_eq!(response["result"]["protocolVersion"], DEFAULT_PROTOCOL_VERSION);
        assert_eq!(server.protocol_version, DEFAULT_PROTOCOL_VERSION);
    }

    #[test]
    fn initialize_accepts_latest_supported_protocol_version() {
        let mut server = McpServer::default();
        let response = server
            .handle_message(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": DEFAULT_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {
                        "name": "github-copilot-developer",
                        "version": "1.0.0"
                    }
                }
            }))
            .expect("initialize should return a response");

        assert_eq!(response["result"]["protocolVersion"], DEFAULT_PROTOCOL_VERSION);
        assert_eq!(server.protocol_version, DEFAULT_PROTOCOL_VERSION);
    }

    #[test]
    fn initialize_accepts_2025_06_18_protocol_version() {
        let mut server = McpServer::default();
        let response = server
            .handle_message(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "specrail-test-client",
                        "version": "0.1.0"
                    }
                }
            }))
            .expect("initialize should return a response");

        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(server.protocol_version, "2025-06-18");
    }

    #[test]
    fn initialize_accepts_2024_11_05_protocol_version() {
        let mut server = McpServer::default();
        let response = server
            .handle_message(json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "specrail-test-client",
                        "version": "0.1.0"
                    }
                }
            }))
            .expect("initialize should return a response");

        assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(server.protocol_version, "2024-11-05");
    }

    #[test]
    fn read_message_accepts_case_insensitive_content_length_header() {
        let payload = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let input = format!(
            "content-length: {}\r\ncontent-type: application/json\r\n\r\n{}",
            payload.len(),
            payload
        );

        let (message, transport) = read_message(&mut Cursor::new(input.into_bytes()))
            .expect("message should parse")
            .expect("message should be present");

        assert_eq!(transport, MessageTransport::ContentLength);
        assert_eq!(message["method"], "ping");
    }

    #[test]
    fn read_message_accepts_raw_json_line_payload() {
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\"}\n";

        let (message, transport) = read_message(&mut Cursor::new(input.as_slice()))
            .expect("message should parse")
            .expect("message should be present");

        assert_eq!(transport, MessageTransport::RawJsonLine);
        assert_eq!(message["method"], "initialize");
    }

    #[test]
    fn write_message_uses_raw_json_line_transport() {
        let mut output = Vec::new();
        let message = json!({"jsonrpc":"2.0","id":1,"result":{}});

        write_message(&mut output, &message, MessageTransport::RawJsonLine)
            .expect("message should write");

        assert_eq!(String::from_utf8(output).unwrap(), "{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{}}\n");
    }
}
