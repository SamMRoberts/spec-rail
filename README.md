# specrail

`specrail` is a Rust CLI for a phase-gated, test-driven workflow. It keeps project state under `.specrail/` and guides work through features, phases, registered tests, implementation, verification, and advancement to the next phase.

## Overview

The tool is designed to help you:

- define a feature before writing code
- break work into explicit phases with goals and path constraints
- register tests up front and track their status
- run an implementation agent only when the phase is ready
- verify each phase with your project's test command
- keep an audit trail of project activity

## How it works

`specrail` stores workflow state in a `.specrail/` directory at the project root:

```text
.specrail/
├── project.yaml
├── specrail.db
├── agents/
└── state/
    └── ledger.jsonl
```

The local database stores solutions, projects, components, features, outcomes,
registered tests, and active workflow state. `project.yaml` remains human-editable,
and `state/ledger.jsonl` remains the append-only audit trail.

Key rules:

- `specrail init` creates the project structure and immediately walks through feature and phase setup.
- `specrail init --no-wizard` creates only the project structure.
- All other commands require an existing `.specrail/` directory and will discover it by walking upward from the current directory.
- `implement` requires an active feature, an active phase, at least one registered test for that phase, and no phase tests left in `planned`.
- `verify` runs the `test_command` from `.specrail/project.yaml`.
- `advance` only works after the active phase has been verified.

## Build and test

From the repository root:

```bash
cargo build
cargo test
```

## Quick start

Initialize a project:

```bash
specrail init
```

If you want only the project files without the walkthrough:

```bash
specrail init --no-wizard
```

Create a feature:

```bash
specrail feature new auth \
  --title "Authentication" \
  --purpose "Authenticate users before they access protected routes." \
  --outcome "Users can sign in" \
  --constraint "Keep login logic inside the auth module"
```

Create phases for the feature:

```bash
specrail phase new auth phase-1 \
  --title "Validation" \
  --goal "Validate credentials and reject bad input." \
  --order 1 \
  --allow "src/auth/**" \
  --forbid "src/billing/**"

specrail phase new auth phase-2 \
  --title "Persistence" \
  --goal "Persist authenticated users." \
  --order 2
```

Register the tests for a phase:

```bash
specrail test add validate-email \
  --feature auth \
  --phase phase-1 \
  --path tests/auth/validate_email.rs \
  --kind unit
```

Or let an AI agent generate the required tests declared in your phase YAML:

```bash
specrail test generate --agent copilot
```

Mark a test as ready for implementation:

```bash
specrail test set-status validate-email written
```

Activate the feature and phase:

```bash
specrail feature activate auth
specrail phase activate auth phase-1
```

Run the workflow:

```bash
specrail implement
specrail verify
specrail advance
```

Inspect progress:

```bash
specrail status
specrail trace --limit 20
```

## Command reference

### Project

- `specrail init`
- `specrail init --no-wizard`

### Features

- `specrail feature new <id> --title <title> --purpose <purpose>`
- `specrail feature list`
- `specrail feature show <id>`
- `specrail feature activate <id>`

Optional feature flags:

- `--outcome`
- `--constraint`
- `--non-goal`
- `--dep`

### Phases

- `specrail phase new <feature_id> <phase_id> --title <title> --goal <goal> --order <n>`
- `specrail phase list <feature_id>`
- `specrail phase show <feature_id> <phase_id>`
- `specrail phase activate <feature_id> <phase_id>`

Optional phase flags:

- `--prereq`
- `--allow`
- `--forbid`
- `--test`

### Tests

- `specrail test add <id> --feature <feature_id> --phase <phase_id> --path <path>`
- `specrail test generate --agent <generic-shell|copilot|codex>`
- `specrail test list [--feature <feature_id>] [--phase <phase_id>]`
- `specrail test set-status <id> <planned|written|passing|failing>`

### Workflow

- `specrail implement [--agent <generic-shell|copilot|codex>]`
- `specrail verify`
- `specrail advance`
- `specrail status`
- `specrail trace [--limit <n>]`

Global flag:

- `-v`, `-vv`, `-vvv` for more verbose logging

## Configuration

`.specrail/project.yaml` contains the project-level settings:

- `version`
- `name`
- `test_command`
- `default_agent`

Default values created by `specrail init` include:

- `test_command: cargo test`
- `default_agent: generic-shell`

## Integrating specrail into another CLI or program

`specrail` works best as the workflow state machine around another AI tool, not as the tool that invents the work on its own. Your outer CLI or program should translate requirements into `specrail` features, phases, and tests, then let `specrail` enforce the handoff points.

Recommended model:

1. Your program collects requirements from the user.
2. Your program turns those requirements into:
   - one `feature`
   - one or more ordered `phase`s
   - one or more tests per phase
3. Your program calls `specrail` commands to persist that plan under `.specrail/`.
4. Your AI coding CLI writes tests and implementation code.
5. `specrail` verifies the phase and decides whether work can advance.

Use the `specrail` CLI as the write interface:

```bash
specrail init
specrail feature new <id> --title <title> --purpose <purpose> ...
specrail phase new <feature> <phase> --title <title> --goal <goal> --order <n> ...
specrail test add <test-id> --feature <feature> --phase <phase> --path <path> ...
specrail test set-status <test-id> written
specrail feature activate <feature>
specrail phase activate <feature> <phase>
```

Use the CLI as the read interface too when your orchestrator needs to inspect progress:

- `specrail status`
- `specrail trace --limit <n>`
- `specrail feature show <feature>`
- `specrail phase show <feature> <phase>`
- `specrail test list --feature <feature> --phase <phase>`

Then run the control loop:

1. **Requirements → specrail**
   - Ask your AI tool to break the requirement into a feature, ordered phases, and tests.
   - Register each artifact with `specrail feature new`, `specrail phase new`, and `specrail test add`.
2. **Test authoring**
   - Hand the active phase goal to your coding CLI and ask it to create the test files for that phase.
   - After each test file exists, mark it ready with `specrail test set-status <id> written`.
3. **Implementation**
   - Activate the feature and phase.
   - Either call your coding CLI directly from your program, or run `specrail implement` to hand the structured phase prompt to an adapter.
4. **Verification**
   - Run `specrail verify`.
   - If verification fails, hand the failure output back to your coding CLI and repeat the implementation step.
5. **Advancement**
   - When verification passes, run `specrail advance`.
   - Repeat until there is no next phase.

### Best integration pattern

For another CLI or automation layer, prefer this ownership split:

- **Your orchestrator** decides what feature, phases, and tests to create.
- **specrail** stores the plan, tracks active state, blocks implementation until tests are registered, and blocks advancement until verification passes.
- **Your AI coding CLI** writes the tests and code.
- **Your project's test command** is the final authority through `specrail verify`.

### Calling another AI CLI from `specrail`

There are three practical options:

- `specrail implement --agent generic-shell`
  - Best when you have a wrapper command that can read `SPECRAIL_PROMPT`, `SPECRAIL_ALLOWED_PATHS`, and `SPECRAIL_FORBIDDEN_PATHS` and then invoke your preferred AI CLI.
  - Configure it with `SPECRAIL_AGENT_CMD`.
- `specrail implement --agent copilot`
  - Uses `copilot -p <prompt>` with the generated implementation brief.
  - Treat this as a prompt/suggestion adapter, not a full autonomous edit pipeline.
- `specrail implement --agent codex`
  - Sends the generated prompt to the `codex` CLI.

Example generic-shell setup:

```bash
export SPECRAIL_AGENT_CMD="your-ai-wrapper"
specrail implement --agent generic-shell
```

Inside `your-ai-wrapper`, read:

- `SPECRAIL_PROMPT` for the full implementation brief
- `SPECRAIL_ALLOWED_PATHS` for editable path hints
- `SPECRAIL_FORBIDDEN_PATHS` for restricted path hints

### GitHub Copilot CLI plugin and MCP server

This repository now ships a first-party Copilot CLI plugin under `.github/plugin/` and a built-in MCP server entrypoint at `specrail mcp-server`.

Prerequisites:

- `specrail` is installed and on `PATH`
- `copilot` is installed and authenticated
- `copilot -p "echo ready"` works in your shell

Install the plugin from the repository root or from GitHub:

```bash
copilot plugin install /path/to/spec-rail
```

Or:

```bash
copilot plugin install SamMRoberts/spec-rail
```

The plugin manifest points Copilot CLI at this MCP server command:

```bash
specrail mcp-server
```

For troubleshooting in VS Code, the workspace MCP config enables stderr logging for the server. Open `MCP: List Servers`, select `SpecRail MCP`, then choose `Show Output` to inspect the live MCP log stream.

After installing, verify that the plugin and MCP server are loaded:

```bash
copilot plugin list
```

In an interactive Copilot CLI session, you should see the plugin skill and the `specrail_*` MCP tools. The MCP server exposes workflow-aware tools such as:

- `specrail_status`
- `specrail_feature_new`
- `specrail_outcome_new`
- `specrail_test_add`
- `specrail_implement`
- `specrail_verify`
- `specrail_advance`

Use those tools instead of editing `.specrail/` files directly.

### Full example: GitHub Copilot CLI

The `copilot` adapter is a good fit when you want `specrail` to own the workflow state and prompt construction, while GitHub Copilot CLI suggests the shell commands to run next.

Prerequisites:

- `specrail` is installed and on `PATH`
- `copilot` is installed and authenticated
- `copilot -p "echo ready"` works in your shell

Example end-to-end flow for a small Rust project:

```bash
mkdir demo-auth
cd demo-auth
cargo init --lib .
specrail init --no-wizard
```

Set Copilot as the default implementation agent:

```yaml
# .specrail/project.yaml
version: 1
name: demo-auth
test_command: cargo test
default_agent: copilot
```

Create a feature, an outcome, and a test entry:

```bash
specrail feature new email-validation \
  --title "Email validation" \
  --purpose "Reject malformed email addresses before account creation." \
  --outcome "Only valid email addresses are accepted."

specrail outcome new email-validation outcome-1 \
  --title "Reject invalid email input" \
  --goal "Add validation logic for malformed email addresses." \
  --order 1 \
  --allow "src/**" \
  --allow "tests/**"

specrail test add rejects-invalid-email \
  --feature email-validation \
  --outcome outcome-1 \
  --path tests/email_validation.rs \
  --kind integration
```

Use Copilot CLI directly to draft the outcome test, then register it as ready:

```bash
copilot -p \
  "Create tests/email_validation.rs with an integration test that proves malformed email addresses are rejected."

# Review or adapt the suggested shell command, run it, then mark the test as written.
specrail test set-status rejects-invalid-email written
```

Activate the feature and outcome, then hand the structured implementation brief to Copilot through `specrail`:

```bash
specrail feature activate email-validation
specrail outcome activate email-validation outcome-1
specrail implement --agent copilot
```

`specrail implement --agent copilot` sends Copilot a prompt containing:

- the feature purpose and outcomes
- the active outcome goal and order
- the allowed and forbidden paths for the outcome
- the registered tests that must pass

Copilot CLI prints a suggested response from the generated prompt. Review it, run or adapt the suggested changes yourself, then continue the gated workflow:

```bash
cargo test
specrail verify
specrail advance
```

For the next outcome, repeat the same loop:

1. write or update the next outcome tests with Copilot CLI
2. mark those tests as `written`
3. run `specrail implement --agent copilot`
4. review and apply Copilot's suggested changes
5. run `specrail verify`
6. run `specrail advance`

### Practical orchestration loop

If you want a Copilot-or-other-agent driven workflow, the safest sequence is:

1. Feed requirements into your own orchestrator.
2. Use the orchestrator to call `specrail feature new` and `specrail phase new`.
3. Ask the coding agent to write the phase tests.
4. Register those tests with `specrail test add`.
5. Mark them `written`.
6. Activate the feature and phase.
7. Ask the coding agent to implement only the active phase.
8. Run `specrail verify`.
9. If tests pass, run `specrail advance`; otherwise loop back to step 7 with the failure output.

This gives you a repeatable contract: the AI can generate code, but `specrail` decides when implementation may start and when the work is allowed to move forward.

## Development notes

- The binary name is `specrail`.
- The default shell-based agent assumes a Unix-like `sh -c` environment.
- Integration tests in `tests/` demonstrate expected CLI behavior and end-to-end workflow.
