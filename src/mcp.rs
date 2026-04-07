use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, bail, Context, Result};
use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::core::{ledger::Ledger, models::TestStatus, repository::Repository};

const DEFAULT_PROTOCOL_VERSION: &str = "2025-11-25";
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
        format!("{}…", &text[..LIMIT])
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
        mcp_debug_log(format!("received message: {}", summarize_for_log(&message.to_string())));
        if let Some(response) = server.handle_message(message) {
            mcp_debug_log(format!("sending response: {}", summarize_for_log(&response.to_string())));
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
        "specrail_feature_list" => tool_feature_list(arguments),
        "specrail_feature_show" => tool_feature_show(arguments),
        "specrail_outcome_list" => tool_outcome_list(arguments),
        "specrail_outcome_show" => tool_outcome_show(arguments),
        "specrail_test_list" => tool_test_list(arguments),
        "specrail_trace" => tool_trace(arguments),
        "specrail_init" => tool_init(arguments),
        "specrail_feature_new" => tool_feature_new(arguments),
        "specrail_feature_activate" => tool_feature_activate(arguments),
        "specrail_outcome_new" => tool_outcome_new(arguments),
        "specrail_outcome_activate" => tool_outcome_activate(arguments),
        "specrail_test_add" => tool_test_add(arguments),
        "specrail_test_generate" => tool_test_generate(arguments),
        "specrail_test_set_status" => tool_test_set_status(arguments),
        "specrail_implement" => tool_implement(arguments),
        "specrail_verify" => tool_verify(arguments),
        "specrail_advance" => tool_advance(arguments),
        other => Ok(tool_error_payload(anyhow!("unknown tool '{other}'"))),
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
                    "cwd": cwd.display().to_string()
                })),
            ));
        }
    };

    let config = repo.load_config()?;
    let state = repo.load_state()?;
    let features = repo.list_features()?;
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

    let active_feature = state.active_feature.clone().unwrap_or_else(|| "(none)".to_string());
    let active_outcome = state.active_outcome.clone().unwrap_or_else(|| "(none)".to_string());
    let project_name = config.name.clone();
    let feature_count = feature_summaries.len();
    let test_count = manifest.tests.len();

    Ok(tool_success_payload(
        format!(
            "Project '{project_name}' is initialized. Active feature: {active_feature}. Active outcome: {active_outcome}. Features: {feature_count}. Tests: {test_count} (passing {passing}, written {written}, planned {planned})."
        ),
        Some(json!({
            "initialized": true,
            "root": repo.root.display().to_string(),
            "config": config,
            "state": state,
            "features": feature_summaries,
            "manifest": manifest,
            "testCounts": {
                "passing": passing,
                "written": written,
                "planned": planned
            }
        })),
    ))
}

fn tool_feature_list(arguments: &Map<String, Value>) -> Result<Value> {
    let repo = discover_repo(arguments)?;
    let features = repo.list_features()?;
    let count = features.len();
    Ok(tool_success_payload(
        format!("Found {count} feature(s)."),
        Some(json!({ "features": features })),
    ))
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
    Ok(tool_success_payload(
        format!("Found {count} outcome(s) for feature '{feature_id}'."),
        Some(json!({ "feature_id": feature_id, "outcomes": outcomes })),
    ))
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

fn tool_feature_new(arguments: &Map<String, Value>) -> Result<Value> {
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
        "new".to_string(),
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

fn tool_feature_activate(arguments: &Map<String, Value>) -> Result<Value> {
    let cwd = resolve_cwd(arguments)?;
    let id = require_string(arguments, "id")?;
    run_cli_tool(&cwd, vec!["feature".to_string(), "activate".to_string(), id])
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
    let cwd = resolve_cwd(arguments)?;
    let mut args = vec!["test".to_string(), "generate".to_string()];
    if let Some(agent) = optional_string(arguments, "agent") {
        args.push("--agent".to_string());
        args.push(agent);
    }
    run_cli_tool(&cwd, args)
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

fn discover_repo(arguments: &Map<String, Value>) -> Result<Repository> {
    let cwd = resolve_cwd(arguments)?;
    Repository::discover(&cwd)
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

fn tool_error_payload(error: impl std::fmt::Display) -> Value {
    tool_payload(error.to_string(), None, true)
}

fn tool_payload(text: String, structured_content: Option<Value>, is_error: bool) -> Value {
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
            "description": "Inspect the current specrail project status and active workflow state.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string", "description": "Workspace or project path to inspect." }
                }
            }
        }),
        json!({
            "name": "specrail_feature_list",
            "description": "List all features registered in the current specrail project.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "specrail_feature_show",
            "description": "Show a feature and its outcomes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string", "description": "Feature identifier." }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "specrail_outcome_list",
            "description": "List outcomes for a feature.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string" }
                },
                "required": ["feature_id"]
            }
        }),
        json!({
            "name": "specrail_outcome_show",
            "description": "Show a single outcome definition.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string" },
                    "outcome_id": { "type": "string" }
                },
                "required": ["feature_id", "outcome_id"]
            }
        }),
        json!({
            "name": "specrail_test_list",
            "description": "List tests from the specrail manifest, optionally filtered by feature or outcome.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string" },
                    "outcome_id": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "specrail_trace",
            "description": "Read the specrail ledger history.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1 }
                }
            }
        }),
        json!({
            "name": "specrail_init",
            "description": "Initialize specrail in the given directory.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "no_wizard": { "type": "boolean", "description": "Defaults to true for MCP usage." }
                }
            }
        }),
        json!({
            "name": "specrail_feature_new",
            "description": "Create a feature in the current specrail project.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" },
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
            "name": "specrail_feature_activate",
            "description": "Activate a feature.",
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
            "name": "specrail_outcome_new",
            "description": "Create an outcome for a feature.",
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
            "name": "specrail_outcome_activate",
            "description": "Activate an outcome for the current feature.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "feature_id": { "type": "string" },
                    "outcome_id": { "type": "string" }
                },
                "required": ["feature_id", "outcome_id"]
            }
        }),
        json!({
            "name": "specrail_test_add",
            "description": "Register a test in the specrail manifest.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" },
                    "feature_id": { "type": "string" },
                    "outcome_id": { "type": "string" },
                    "path": { "type": "string" },
                    "kind": { "type": "string", "enum": ["unit", "integration", "e2e"] },
                    "purpose_refs": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["id", "feature_id", "outcome_id", "path"]
            }
        }),
        json!({
            "name": "specrail_test_generate",
            "description": "Generate required tests using the configured or selected agent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "agent": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "specrail_test_set_status",
            "description": "Update a test status in the manifest.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "id": { "type": "string" },
                    "status": { "type": "string", "enum": ["planned", "written", "passing", "failing"] }
                },
                "required": ["id", "status"]
            }
        }),
        json!({
            "name": "specrail_implement",
            "description": "Run the implementation agent for the active outcome.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "agent": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "specrail_verify",
            "description": "Run the project's configured verification command for the active outcome.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" }
                }
            }
        }),
        json!({
            "name": "specrail_advance",
            "description": "Advance from the verified outcome to the next one.",
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
