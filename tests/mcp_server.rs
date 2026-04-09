use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

mod support;

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
        Self::spawn_with_env(cwd, &[])
    }

    fn spawn_with_env(cwd: &Path, envs: &[(&str, &str)]) -> Self {
        let mut command = Command::new(cargo_bin("specrail"));
        command
            .current_dir(cwd)
            .args(["mcp-server"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (key, value) in envs {
            command.env(key, value);
        }

        let mut child = command.spawn().expect("spawn specrail mcp-server");

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
    assert!(tool_names.contains(&"specrail_solution_list"));
    assert!(tool_names.contains(&"specrail_project_list"));
    assert!(tool_names.contains(&"specrail_component_list"));
    assert!(tool_names.contains(&"specrail_init"));
    assert!(tool_names.contains(&"specrail_verify"));
    assert!(tool_names.contains(&"specrail_outcome_unverify"));
    assert!(tool_names.contains(&"specrail_outcome_add_required_test"));
    assert!(tool_names.contains(&"specrail_outcome_test_review"));
    assert!(tool_names.contains(&"specrail_test_suggest"));
    assert!(tool_names.contains(&"specrail_test_apply_generated"));
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
        .contains("specrail_feature_navigate"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("specrail_outcome_unverify"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("specrail_implement"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("specrail_verify"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("testReviewTool"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("hasTestGaps"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("hero-panel"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Workspace overview"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Solution / Project / Component overview"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Related tests"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Preview AI Suggestions"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Implementation prompt ready"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Copy Prompt"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Start Editor Chat"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("command:"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("workbench.action.chat.open"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Implementation prompt copied to clipboard."));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("btn-status-progress"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("In progress"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("data-tool-action"));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("document.addEventListener(\"click\""));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Pointer down on "));
    assert!(feature_picker_html["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Dispatching tool action:"));

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
    assert_eq!(
        status_after["result"]["structuredContent"]["solutions"][0]["id"],
        "default-solution"
    );
    assert_eq!(
        status_after["result"]["structuredContent"]["projects"][0]["id"],
        "default-project"
    );
    assert_eq!(
        status_after["result"]["structuredContent"]["components"][0]["id"],
        "default-component"
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
    assert_eq!(features[0]["solution_id"], "default-solution");
    assert_eq!(features[0]["project_id"], "default-project");
    assert_eq!(features[0]["component_id"], "default-component");

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
    let solutions = client.request(
        "tools/call",
        json!({
            "name": "specrail_solution_list",
            "arguments": {}
        }),
    );
    assert_eq!(
        solutions["result"]["structuredContent"]["solutions"][0]["id"],
        "default-solution"
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
                "order": 1,
                "required_tests": ["auth-login-rs", "auth-mfa-rs"],
                "required_test_files": ["tests/auth/login.rs", "tests/auth/mfa.rs"]
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_add",
            "arguments": {
                "id": "auth-login-rs",
                "feature_id": "auth",
                "outcome_id": "login",
                "path": "tests/auth/login.rs",
                "kind": "unit",
                "purpose_refs": []
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_add",
            "arguments": {
                "id": "auth-login-smoke-rs",
                "feature_id": "auth",
                "outcome_id": "login",
                "path": "tests/auth/login_smoke.rs",
                "kind": "unit",
                "purpose_refs": []
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_edit",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "title": "User login",
                "goal": "Let a user sign in with valid credentials.",
                "order": 1,
                "prerequisites": [],
                "allowed_paths": [],
                "forbidden_paths": [],
                "required_tests": ["auth-login-rs", "auth-mfa-rs"],
                "required_test_files": ["tests/auth/login.rs", "tests/auth/mfa.rs"]
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
    assert!(features.iter().all(|feature| feature["pathLabel"] == "default-solution/default-project/default-component"));
    let auth_feature = features
        .iter()
        .find(|feature| feature["id"] == "auth")
        .unwrap();
    assert_eq!(auth_feature["totalTestCount"], 2);
    assert_eq!(auth_feature["plannedTestCount"], 2);
    assert_eq!(auth_feature["undeclaredOutcomeCount"], 1);
    assert_eq!(auth_feature["hasUndeclaredTests"], json!(true));
    assert_eq!(auth_feature["implementStatus"]["tone"], "warning");
    assert_eq!(auth_feature["implementStatus"]["label"], "Needed");
    assert_eq!(auth_feature["verifyStatus"]["tone"], "muted");
    assert_eq!(auth_feature["verifyStatus"]["label"], "Waiting");
    assert_eq!(
        feature_picker["result"]["structuredContent"]["solutions"][0]["id"],
        "default-solution"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["activeSolutionId"],
        "default-solution"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["activeProjectId"],
        "default-project"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["activeComponentId"],
        "default-component"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["nextActions"]["selectFeatureTool"],
        "specrail_feature_navigate"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["nextActions"]["activateSolutionTool"],
        "specrail_solution_activate"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["nextActions"]["activateProjectTool"],
        "specrail_project_activate"
    );
    assert_eq!(
        feature_picker["result"]["structuredContent"]["nextActions"]["activateComponentTool"],
        "specrail_component_activate"
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
        outcome_picker["result"]["_meta"]["ui"]["resourceUri"],
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
    assert_eq!(outcomes[0]["goal"], "Let a user sign in with valid credentials.");
    assert_eq!(outcomes[0]["requiredTestCount"], 2);
    assert_eq!(outcomes[0]["missingRequiredTestCount"], 1);
    assert_eq!(outcomes[0]["plannedTestCount"], 1);
    assert_eq!(outcomes[0]["writtenTestCount"], 0);
    assert_eq!(outcomes[0]["failingTestCount"], 0);
    assert_eq!(outcomes[0]["undeclaredTestCount"], 1);
    assert_eq!(outcomes[0]["hasTestGaps"], json!(true));
    let missing_required = outcomes[0]["testReview"]["missing_required_tests"]
        .as_array()
        .unwrap();
    assert!(missing_required.iter().any(|value| value == "auth-mfa-rs"));
    assert_eq!(
        outcomes[0]["testReview"]["missing_required_test_files"]
        .as_array()
        .unwrap()
        .len(),
        0
    );
    let undeclared_tests = outcomes[0]["testReview"]["undeclared_tests"]
        .as_array()
        .unwrap();
    assert!(undeclared_tests
        .iter()
        .any(|value| value["path"] == "tests/auth/login_smoke.rs"));
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
        outcome_picker["result"]["structuredContent"]["nextActions"]["unverifyOutcomeTool"],
        "specrail_outcome_unverify"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["testReviewTool"],
        "specrail_outcome_test_review"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["testSuggestTool"],
        "specrail_test_suggest"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["testAddTool"],
        "specrail_test_add"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["testSetStatusTool"],
        "specrail_test_set_status"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["nextActions"]["implementTool"],
        "specrail_implement"
    );
    assert_eq!(
        outcome_picker["result"]["structuredContent"]["suggestedNewOutcomeOrder"],
        2
    );
    assert!(outcomes[0]["availableActions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "activate"));
    assert!(outcomes[0]["availableActions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "tests"));
    assert!(outcomes[0]["availableActions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "generate_tests"));

    let review = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_test_review",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login"
            }
        }),
    );
    assert_eq!(review["result"]["structuredContent"]["review"]["has_gaps"], json!(true));
    assert!(review["result"]["structuredContent"]["review"]["planned_required_tests"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "auth-login-rs"));
    assert_eq!(
        review["result"]["structuredContent"]["review"]["planned_required_test_files"][0],
        "tests/auth/login.rs"
    );
    assert_eq!(
        review["result"]["structuredContent"]["review"]["suggested_test_paths"][0],
        "tests/auth/login.rs"
    );

    client.shutdown();
}

#[test]
fn mcp_feature_navigate_scopes_features_to_active_hierarchy() {
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
            "name": "specrail_solution_new",
            "arguments": {
                "id": "platform",
                "title": "Platform",
                "purpose": "Shared delivery surface."
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_project_new",
            "arguments": {
                "solution_id": "platform",
                "id": "api",
                "title": "API",
                "purpose": "Backend services."
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_component_new",
            "arguments": {
                "project_id": "api",
                "id": "billing",
                "title": "Billing",
                "purpose": "Charge subscriptions."
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_component_new",
            "arguments": {
                "project_id": "api",
                "id": "identity",
                "title": "Identity",
                "purpose": "Authenticate requests."
            }
        }),
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_new",
            "arguments": {
                "id": "invoice-retries",
                "component_id": "billing",
                "title": "Invoice retries",
                "purpose": "Retry failed invoice collection."
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_new",
            "arguments": {
                "id": "token-refresh",
                "component_id": "identity",
                "title": "Token refresh",
                "purpose": "Refresh expired access tokens."
            }
        }),
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_component_activate",
            "arguments": {
                "id": "billing"
            }
        }),
    );

    let billing_picker = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_navigate",
            "arguments": {}
        }),
    );
    let billing_features = billing_picker["result"]["structuredContent"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(billing_picker["result"]["structuredContent"]["activeSolutionId"], "platform");
    assert_eq!(billing_picker["result"]["structuredContent"]["activeProjectId"], "api");
    assert_eq!(billing_picker["result"]["structuredContent"]["activeComponentId"], "billing");
    assert_eq!(billing_features.len(), 1);
    assert_eq!(billing_features[0]["id"], "invoice-retries");
    assert_eq!(
        billing_picker["result"]["structuredContent"]["visibleProjects"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        billing_picker["result"]["structuredContent"]["visibleComponents"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_component_activate",
            "arguments": {
                "id": "identity"
            }
        }),
    );

    let identity_picker = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_navigate",
            "arguments": {}
        }),
    );
    let identity_features = identity_picker["result"]["structuredContent"]["features"]
        .as_array()
        .unwrap();
    assert_eq!(identity_picker["result"]["structuredContent"]["activeComponentId"], "identity");
    assert_eq!(identity_features.len(), 1);
    assert_eq!(identity_features[0]["id"], "token-refresh");

    client.shutdown();
}

#[test]
fn mcp_test_suggest_previews_without_writing_files() {
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
                "order": 1,
                "required_tests": ["auth-login-valid-credentials"],
                "required_test_files": ["tests/auth/login.rs"]
            }
        }),
    );

    let preview = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_suggest",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "agent": "generic-shell"
            }
        }),
    );

    assert_eq!(
        preview["result"]["structuredContent"]["delegation"]["scope_label"],
        "auth:login"
    );
    assert_eq!(
        preview["result"]["structuredContent"]["delegation"]["agent"],
        "generic-shell"
    );
    assert!(preview["result"]["structuredContent"]["delegation"]["prompt"]
        .as_str()
        .unwrap()
        .contains("User login"));
    assert_eq!(
        preview["result"]["structuredContent"]["delegation"]["allow_path_discovery"],
        json!(true)
    );
    assert!(!dir.path().join("tests/auth/login.rs").exists());

    let tests = support::tests(&dir);
    assert!(!tests.iter().any(|test| test.path == "tests/auth/login.rs"));

    client.shutdown();
}

#[test]
fn mcp_test_generate_can_target_a_specific_outcome_without_activation() {
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
                "order": 1,
                "required_tests": ["logs_in"],
                "required_test_files": ["tests/auth/login.rs"]
            }
        }),
    );

    let generation = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_generate",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "agent": "generic-shell"
            }
        }),
    );

    assert_eq!(generation["result"]["isError"], json!(false));
    assert_eq!(
        generation["result"]["structuredContent"]["delegation"]["scope_label"],
        "auth:login"
    );
    assert_eq!(
        generation["result"]["structuredContent"]["delegation"]["apply_tool"],
        "specrail_test_apply_generated"
    );
    assert!(!dir.path().join("tests/auth/login.rs").exists());

    let apply = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_apply_generated",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "agent": "generic-shell",
                "generated_tests": [
                    {
                        "feature_id": "auth",
                        "outcome_id": "login",
                        "id": "auth-login-valid-credentials",
                        "name": "logs_in",
                        "path": "tests/auth/login.rs",
                        "kind": "unit",
                        "purpose_refs": ["goal:Let a user sign in with valid credentials."],
                        "content": "#[test]\nfn logs_in() {\n    assert!(true);\n}\n"
                    }
                ]
            }
        }),
    );

    assert_eq!(apply["result"]["isError"], json!(false));
    assert_eq!(
        apply["result"]["structuredContent"]["generation"]["generated_count"],
        1
    );
    assert!(dir.path().join("tests/auth/login.rs").exists());

    let tests = support::tests(&dir);
    assert!(tests.iter().any(|test| {
        test.id == "auth-login-valid-credentials"
            && test.name == "logs_in"
            && test.path == "tests/auth/login.rs"
            && test.status == "written"
    }));

    assert!(support::outcome_required_tests(&dir, "auth", "login")
        .contains(&"auth-login-valid-credentials".to_string()));
    assert!(support::outcome_required_test_files(&dir, "auth", "login")
        .contains(&"tests/auth/login.rs".to_string()));

    client.shutdown();
}

#[test]
fn mcp_test_generate_bootstraps_required_tests_for_new_outcome() {
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

    let generation = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_generate",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "agent": "generic-shell"
            }
        }),
    );

    assert_eq!(generation["result"]["isError"], json!(false));
    assert!(!dir.path().join("tests/auth/login.rs").exists());

    let apply = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_apply_generated",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "agent": "generic-shell",
                "generated_tests": [
                    {
                        "feature_id": "auth",
                        "outcome_id": "login",
                        "id": "auth-login-valid-credentials",
                        "name": "logs_in",
                        "path": "tests/auth/login.rs",
                        "kind": "unit",
                        "purpose_refs": ["goal:Let a user sign in with valid credentials."],
                        "content": "#[test]\nfn logs_in() {\n    assert!(true);\n}\n"
                    }
                ]
            }
        }),
    );

    assert_eq!(apply["result"]["isError"], json!(false));
    assert!(dir.path().join("tests/auth/login.rs").exists());

    assert!(support::outcome_required_tests(&dir, "auth", "login")
        .contains(&"auth-login-valid-credentials".to_string()));
    assert!(support::outcome_required_test_files(&dir, "auth", "login")
        .contains(&"tests/auth/login.rs".to_string()));

    let tests = support::tests(&dir);
    assert!(tests.iter().any(|test| {
        test.id == "auth-login-valid-credentials"
            && test.name == "logs_in"
            && test.path == "tests/auth/login.rs"
            && test.status == "written"
    }));

    client.shutdown();
}

#[test]
fn mcp_test_generate_promotes_existing_planned_test_to_written() {
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
                "order": 1,
                "required_tests": ["auth-login-valid-credentials"],
                "required_test_files": ["tests/auth/login.rs"]
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_add",
            "arguments": {
                "id": "auth-login-valid-credentials",
                "feature_id": "auth",
                "outcome_id": "login",
                "path": "tests/auth/login.rs",
                "kind": "unit",
                "purpose_refs": []
            }
        }),
    );

    let generation = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_generate",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "agent": "generic-shell"
            }
        }),
    );

    assert_eq!(generation["result"]["isError"], json!(false));

    let apply = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_apply_generated",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "agent": "generic-shell",
                "generated_tests": [
                    {
                        "feature_id": "auth",
                        "outcome_id": "login",
                        "id": "auth-login-valid-credentials",
                        "name": "logs_in",
                        "path": "tests/auth/login.rs",
                        "kind": "unit",
                        "purpose_refs": ["goal:Let a user sign in with valid credentials."],
                        "content": "#[test]\nfn logs_in() {\n    assert!(true);\n}\n"
                    }
                ]
            }
        }),
    );

    assert_eq!(apply["result"]["isError"], json!(false));

    let tests = support::tests(&dir);
    assert!(tests.iter().any(|test| {
        test.id == "auth-login-valid-credentials"
            && test.name == "logs_in"
            && test.path == "tests/auth/login.rs"
            && test.status == "written"
    }));

    client.shutdown();
}

#[test]
fn mcp_implement_runs_cli_implement_command() {
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
                "order": 1,
                "required_tests": ["auth-login-valid-credentials"],
                "required_test_files": ["tests/auth/login.rs"]
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_add",
            "arguments": {
                "id": "auth-login-valid-credentials",
                "feature_id": "auth",
                "outcome_id": "login",
                "path": "tests/auth/login.rs",
                "kind": "unit",
                "purpose_refs": []
            }
        }),
    );
    fs::create_dir_all(dir.path().join("tests/auth")).unwrap();
    fs::write(
        dir.path().join("tests/auth/login.rs"),
        "#[test]\nfn logs_in() {\n    assert!(true);\n}\n",
    )
    .unwrap();
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
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_activate",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login"
            }
        }),
    );

    let project_yaml_path = dir.path().join(".specrail/project.yaml");
    let project_yaml = fs::read_to_string(&project_yaml_path).unwrap();
    fs::write(
        &project_yaml_path,
        project_yaml.replace("default_agent: copilot", "default_agent: generic-shell"),
    )
    .unwrap();

    let implement = client.request(
        "tools/call",
        json!({
            "name": "specrail_implement",
            "arguments": {}
        }),
    );

    // MCP implement must prepare an in-chat delegation instead of spawning the CLI agent.
    let sc = &implement["result"]["structuredContent"];
    assert!(
        sc.get("delegation").is_some(),
        "expected structuredContent.delegation, got: {sc}"
    );
    assert!(
        sc["delegation"]["prompt"].as_str().unwrap_or("").contains("User login"),
        "expected implementation prompt to mention the active outcome; got: {sc}"
    );
    assert_eq!(
        sc["delegation"]["agent"], json!("generic-shell")
    );
    assert!(
        implement["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .contains("current VS Code chat"),
        "expected implement text to describe in-chat delegation; got: {}",
        implement["result"]["content"][0]["text"]
    );

    client.shutdown();
}

#[test]
fn mcp_outcome_add_required_test_promotes_undeclared_related_test() {
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
                "order": 1,
                "required_tests": ["auth-password-rs"],
                "required_test_files": ["tests/auth/password.rs"]
            }
        }),
    );
    let _ = client.request(
        "tools/call",
        json!({
            "name": "specrail_test_add",
            "arguments": {
                "id": "auth-login-rs",
                "feature_id": "auth",
                "outcome_id": "login",
                "path": "tests/auth/login.rs",
                "kind": "unit",
                "purpose_refs": []
            }
        }),
    );

    support::remove_required_test(&dir, "auth", "login", "tests/auth/login.rs");

    let review_before = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_test_review",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login"
            }
        }),
    );
    assert_eq!(
        review_before["result"]["structuredContent"]["review"]["has_no_related_tests"],
        json!(true)
    );

    let add_required = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_add_required_test",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login",
                "test_id": "auth-login-rs",
                "path": "tests/auth/login.rs"
            }
        }),
    );
    assert_eq!(add_required["result"]["isError"], json!(false));
    assert_eq!(add_required["result"]["structuredContent"]["added"]["test_id"], json!(false));
    assert_eq!(add_required["result"]["structuredContent"]["added"]["test_file"], json!(false));

    assert!(support::outcome_required_tests(&dir, "auth", "login")
        .contains(&"auth-login-rs".to_string()));
    assert!(support::outcome_required_test_files(&dir, "auth", "login").is_empty());

    let review_after = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_test_review",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login"
            }
        }),
    );
    assert!(review_after["result"]["structuredContent"]["review"]["undeclared_tests"]
        .as_array()
        .unwrap()
        .is_empty());

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

    support::set_outcome_status(&dir, "auth", "login", "completed");

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
fn mcp_outcome_unverify_resets_verified_outcome_to_pending() {
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

    support::set_outcome_status(&dir, "auth", "login", "verified");

    let reset = client.request(
        "tools/call",
        json!({
            "name": "specrail_outcome_unverify",
            "arguments": {
                "feature_id": "auth",
                "outcome_id": "login"
            }
        }),
    );
    assert_eq!(reset["result"]["isError"], json!(false));
    assert_eq!(reset["result"]["structuredContent"]["outcome"]["status"], "pending");

    let navigate = client.request(
        "tools/call",
        json!({
            "name": "specrail_feature_navigate",
            "arguments": {
                "feature_id": "auth"
            }
        }),
    );
    let outcomes = navigate["result"]["structuredContent"]["outcomes"]
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
        "specrail-setup"
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
        "specrail-plan-features"
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
        "specrail-prepare-tests"
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

    fs::create_dir_all(dir.path().join("tests/auth")).unwrap();
    fs::write(
        dir.path().join("tests/auth/login.rs"),
        "#[test]\nfn auth_login_valid_credentials() {}\n",
    )
    .unwrap();

    let status_planned_tests = client.request(
        "tools/call",
        json!({
            "name": "specrail_status",
            "arguments": {}
        }),
    );
    assert_eq!(
        status_planned_tests["result"]["structuredContent"]["workflow"]["recommended_skill"],
        "specrail-prepare-tests"
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
        "specrail-run-workflow"
    );
    assert!(
        status_ready["result"]["structuredContent"]["workflow"]["next_tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool == "specrail_outcome_test_review")
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
