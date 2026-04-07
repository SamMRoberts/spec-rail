use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use tempfile::TempDir;

struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    fn spawn(cwd: &Path) -> Self {
        let mut child = Command::new(cargo_bin("specrail"))
            .current_dir(cwd)
            .args(["mcp-server"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn specrail mcp-server");

        let stdin = child.stdin.take().expect("capture stdin");
        let stdout = child.stdout.take().expect("capture stdout");

        Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
        }
    }

    fn initialize(&mut self) -> Value {
        let response = self.request(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "specrail-test-client",
                    "version": "0.1.0"
                }
            }),
        );
        self.notify("notifications/initialized", json!({}));
        response
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;

        write_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params
            }),
        );

        let response = read_message(&mut self.stdout);
        assert_eq!(response["id"], json!(id));
        response
    }

    fn notify(&mut self, method: &str, params: Value) {
        write_message(
            &mut self.stdin,
            &json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params
            }),
        );
    }

    fn shutdown(&mut self) {
        let _ = self.request("shutdown", json!({}));
        self.notify("exit", json!({}));
        let _ = self.child.wait();
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn mcp_server_lists_tools_and_initializes_project() {
    let dir = TempDir::new().unwrap();
    let mut client = McpClient::spawn(dir.path());

    let initialize = client.initialize();
    assert_eq!(initialize["result"]["serverInfo"]["name"], "specrail");
    assert!(initialize["result"]["capabilities"]["tools"].is_object());
    assert!(initialize["result"]["capabilities"]["resources"].is_object());
    assert_eq!(
        initialize["result"]["capabilities"]["extensions"]["io.modelcontextprotocol/ui"]
            ["mimeTypes"][0],
        "text/html;profile=mcp-app"
    );

    let tools = client.request("tools/list", json!({}));
    let tool_names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(tool_names.contains(&"specrail_status"));
    assert!(tool_names.contains(&"specrail_feature_navigate"));
    assert!(tool_names.contains(&"specrail_init"));
    assert!(tool_names.contains(&"specrail_verify"));
    let feature_navigate = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == "specrail_feature_navigate")
        .unwrap();
    assert_eq!(
        feature_navigate["_meta"]["ui"]["resourceUri"],
        "ui://specrail/feature-navigate"
    );

    let resources = client.request("resources/list", json!({}));
    let feature_picker = resources["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["uri"] == "ui://specrail/feature-navigate")
        .unwrap();
    assert_eq!(feature_picker["mimeType"], "text/html;profile=mcp-app");

    let feature_picker_html = client.request(
        "resources/read",
        json!({
            "uri": "ui://specrail/feature-navigate"
        }),
    );
    assert_eq!(
        feature_picker_html["result"]["contents"][0]["mimeType"],
        "text/html;profile=mcp-app"
    );
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Select or edit a feature"));

    let status_before = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_before["result"]["structuredContent"]["initialized"],
        json!(false)
    );

    let init = client.request(
        "tools/call",
        json!({
            "name": "specrail_init",
            "arguments": {
                "no_wizard": true
            }
        }),
    );
    assert_eq!(init["result"]["isError"], json!(false));
    assert!(dir.path().join(".specrail").is_dir());

    let status_after = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_after["result"]["structuredContent"]["initialized"],
        json!(true)
    );
    assert_eq!(
        status_after["result"]["structuredContent"]["manifest"]["tests"]
            .as_array()
            .unwrap()
            .len(),
        0
    );

    client.shutdown();
}

#[test]
fn mcp_server_can_create_and_read_feature_state() {
    let dir = TempDir::new().unwrap();
    let mut client = McpClient::spawn(dir.path());
    client.initialize();

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_init",
            "arguments": {
                "no_wizard": true
            }
        }),
    );

    let create_feature = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_new",
            "arguments": {
                "id": "auth",
                "title": "Authentication",
                "purpose": "Authenticate users before protected routes.",
                "outcomes": ["Users can sign in"]
            }
        }),
    );
    assert_eq!(create_feature["result"]["isError"], json!(false));

    let feature_list = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_list",
            "arguments": {}
        }),
    );
    let features = feature_list["result"]["structuredContent"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(features.len(), 1);
    assert_eq!(features[0]["id"], "auth");

    let feature_show = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_show",
            "arguments": {
                "id": "auth"
            }
        }),
    );
    assert_eq!(
        feature_show["result"]["structuredContent"]["feature"]["title"],
        "Authentication"
    );

    client.shutdown();
}

#[test]
fn mcp_server_can_navigate_features_and_outcomes_for_selection() {
    let dir = TempDir::new().unwrap();
    let mut client = McpClient::spawn(dir.path());
    client.initialize();

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_init",
            "arguments": {
                "no_wizard": true
            }
        }),
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_new",
            "arguments": {
                "id": "auth",
                "title": "Authentication",
                "purpose": "Authenticate users before protected routes."
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_new",
            "arguments": {
                "id": "billing",
                "title": "Billing",
                "purpose": "Charge customers for subscriptions."
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_new",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "title": "User login",
                "goal": "Let a user sign in with valid credentials.",
                "order": 1
            }
        }),
    );

    let feature_picker = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_navigate",
            "arguments": {}
        }),
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["mode"],
        "feature_selection"
    );
    assert_eq!(
        feature_picker["result"]["_meta"]["ui"]["resourceUri"],
        "ui://specrail/feature-navigate"
    );
    let features = feature_picker["result"]["structuredContent"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(features.len(), 2);
    assert!(features.iter().any(|feature| feature["id"] == "auth"));
    assert_eq!(
        feature_picker["result"]["structuredContent"]["nextActions"]["selectFeatureTool"],
        "specrail_feature_navigate"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["nextActions"]["editFeatureTool"],
        "specrail_feature_edit"
    );

    let outcome_picker = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_navigate",
            "arguments": {
                "feature_id": "auth"
            }
        }),
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["mode"],
        "outcome_selection"
    );
    assert_eq!(
        outcome_picker["result"]["_meta"]["ui/resourceUri"],
        "ui://specrail/feature-navigate"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["selectedFeature"]["id"],
        "auth"
    );
    let outcomes = outcome_picker["result"]["structuredContent"]["outcomes"]
        .as_array()
        .unwrap();
    assert_eq!(outcomes.len(), 1);
    assert_eq!(outcomes[0]["id"], "login");
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["selectOutcomeTool"],
        "specrail_outcome_activate"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["createOutcomeTool"],
        "specrail_outcome_new"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["editOutcomeTool"],
        "specrail_outcome_edit"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["suggestedNewOutcomeOrder"],
        2
    );

    client.shutdown();
}

#[test]
fn mcp_feature_navigate_refreshes_outcome_status_after_edit() {
    let dir = TempDir::new().unwrap();
    let mut client = McpClient::spawn(dir.path());
    client.initialize();

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_init",
            "arguments": {
                "no_wizard": true
            }
        }),
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_new",
            "arguments": {
                "id": "auth",
                "title": "Authentication",
                "purpose": "Authenticate users before protected routes."
            }
        }),
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_new",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "title": "User login",
                "goal": "Let a user sign in with valid credentials.",
                "order": 1
            }
        }),
    );

    let outcome_path = dir
        .path()
        .join(".specrail/outcomes/auth/login.yaml");
    let updated = fs::read_to_string(&outcome_path)
        .unwrap()
        .replace("status: pending", "status: completed");
    fs::write(&outcome_path, updated).unwrap();

    let edit = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_edit",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "title": "User login",
                "goal": "Let a user sign in with valid credentials and MFA.",
                "order": 1,
                "prerequisites": [],
                "allowed_paths": [],
                "forbidden_paths": [],
                "required_tests": []
            }
        }),
    );
    assert_eq!(edit["result"]["isError"], json!(false));

    let refreshed = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_navigate",
            "arguments": {
                "feature_id": "auth"
            }
        }),
    );

    let outcomes = refreshed["result"]["structuredContent"]["outcomes"]
        .as_array()
        .unwrap();
    assert_eq!(outcomes[0]["status"], "pending");

    client.shutdown();
}

#[test]
fn mcp_status_guides_the_end_to_end_tdd_flow() {
    let dir = TempDir::new().unwrap();
    let mut client = McpClient::spawn(dir.path());
    client.initialize();

    let status_before_init = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_before_init["result"]["structuredContent"]["workflow"]["recommended_skill"],
        "specrail-init"
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_init",
            "arguments": {
                "no_wizard": true
            }
        }),
    );

    let status_after_init = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_after_init["result"]["structuredContent"]["workflow"]["recommended_skill"],
        "specrail-workflow"
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_new",
            "arguments": {
                "id": "auth",
                "title": "Authentication",
                "purpose": "Authenticate users before protected routes."
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_new",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "title": "User login",
                "goal": "Let a user sign in with valid credentials.",
                "order": 1
            }
        }),
    );

    let status_needs_tests = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_needs_tests["result"]["structuredContent"]["workflow"]["recommended_skill"],
        "specrail-testing"
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_add",
            "arguments": {
                "id": "auth-login-valid-credentials",
                "feature_id": "auth",
                "outcome_id": "login",
                "path": "tests/auth/login.rs"
            }
        }),
    );

    let status_planned_tests = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_planned_tests["result"]["structuredContent"]["workflow"]["recommended_skill"],
        "specrail-testing"
    );
    assert!(status_planned_tests["result"]["structuredContent"]["workflow"]["blockers"][0]
        .as_str()
        .unwrap()
        .contains("planned status"));

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_set_status",
            "arguments": {
                "id": "auth-login-valid-credentials",
                "status": "written"
            }
        }),
    );

    let status_ready = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_ready["result"]["structuredContent"]["workflow"]["recommended_skill"],
        "specrail-activation"
    );
    assert_eq!(
        status_ready["result"]["structuredContent"]["workflow"]["candidate_feature_id"],
        "auth"
    );
    assert_eq!(
        status_ready["result"]["structuredContent"]["workflow"]["candidate_outcome_id"],
        "login"
    );

    client.shutdown();
}

fn write_message(writer: &mut impl Write, message: &Value) {
    let payload = serde_json::to_vec(message).unwrap();
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len()).unwrap();
    writer.write_all(&payload).unwrap();
    writer.flush().unwrap();
}

fn read_message(reader: &mut impl BufRead) -> Value {
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).unwrap();
        assert!(read > 0, "unexpected EOF while reading MCP headers");

        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }

        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().unwrap());
        }
    }

    let mut payload = vec![0; content_length.expect("missing Content-Length header")];
    reader.read_exact(&mut payload).unwrap();
    serde_json::from_slice(&payload).unwrap()
}
