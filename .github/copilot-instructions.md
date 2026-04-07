# Copilot instructions for `specrail`

## What this repository is

`specrail` is a Rust CLI for an outcome-gated, test-driven workflow. The binary is `specrail`, and nearly all behavior revolves around creating and updating files under `.specrail/`.

For a first pass, read these files in order:

1. `src/main.rs` - top-level command dispatch
2. `src/cli.rs` - CLI surface and arguments
3. `src/core/models.rs` - persisted domain model
4. `src/core/repository.rs` - canonical `.specrail` path resolver and YAML I/O
5. `src/commands/implement.rs`, `src/commands/verify.rs`, `src/commands/advance.rs` - main workflow loop
6. `src/mcp.rs` - built-in stdio MCP server
7. `tests/*.rs` - integration coverage and expected CLI behavior

## Repository layout

- `src/commands/`: command handlers
- `src/core/`: config, repository access, state, ledger, models
- `src/policy/`: outcome and manifest validation rules
- `src/agents/`: adapter layer for `generic-shell`, `copilot`, and `codex`
- `src/prompts/`: prompt construction for `implement`
- `src/runtime/`: filesystem and process helpers
- `src/mcp.rs`: stdio MCP server (`specrail mcp-server`)
- `.github/plugin/`: installable Copilot CLI plugin (`.mcp.json` + skills)
- `tests/`: integration tests using `assert_cmd`, `tempfile`, and `predicates`

Generated project state lives under `.specrail/`:

- `.specrail/project.yaml`
- `.specrail/features/*.yaml`
- `.specrail/outcomes/<feature>/*.yaml`
- `.specrail/tests/manifest.yaml`
- `.specrail/state/current.yaml`
- `.specrail/state/ledger.jsonl`

Treat `src/core/repository.rs` as the source of truth for where project files belong.

## Normal workflow

The intended CLI flow is:

1. `specrail init` (pass `--no-wizard` to skip the interactive onboarding walkthrough)
2. `specrail feature new ...`
3. `specrail outcome new ...`
4. `specrail test add ...`
5. `specrail test set-status <id> written`
6. `specrail feature activate <id>`
7. `specrail outcome activate <feature> <outcome>`
8. `specrail implement`
9. `specrail verify`
10. `specrail advance`

Additional commands:

- `specrail status` — show a project status dashboard (features, outcomes, test counts)
- `specrail trace [--limit N]` — show the append-only audit ledger
- `specrail test generate` — generate required tests using an AI agent
- `specrail mcp-server` — run the specrail MCP server over stdio (also aliased as `mcpserver`)

Important gating rules:

- All commands except `init` require an existing `.specrail/` directory; discovery walks upward from the current directory.
- `implement` requires an active feature, an active outcome, at least one registered test for that outcome, and no outcome tests left in `planned` status.
- `verify` uses `project.yaml:test_command` from the project root and marks the outcome `verified` or `failed`.
- `advance` only works from a `verified` outcome and activates the outcome whose `order` is current `order + 1`.

## Coding conventions

- Use `anyhow::Result` with contextual errors via `.context(...)` / `.with_context(...)`.
- Persisted data is serialized with `serde_yaml`; ledger entries are JSON Lines in `ledger.jsonl`.
- Keep changes small and aligned with the existing module layout.
- Reuse `Repository` helpers instead of hardcoding `.specrail` paths.
- Integration tests are preferred over unit tests in this repo; follow the existing `tests/*.rs` pattern with `Command::cargo_bin("specrail")` and `TempDir`.

## Validation commands

Run these from the repository root:

```bash
cargo build
cargo test
```

These commands pass in the current repository state and are the baseline validation to run before and after changes.

## Agent-specific behavior

- The default agent is configured in `.specrail/project.yaml`; by default it is `generic-shell`.
- `generic-shell` passes the prompt in `SPECRAIL_PROMPT` and path hints in `SPECRAIL_ALLOWED_PATHS` / `SPECRAIL_FORBIDDEN_PATHS`.
- `copilot` shells out to `gh copilot suggest -t shell` and returns Copilot's suggestion output.
- `codex` is also available as a third agent option.
- Allowed and forbidden paths are currently prompt/env hints, not enforced write protections.
- Shell execution uses `sh -c`, so agent and test commands assume a Unix-like shell environment.

## MCP server and Copilot plugin

- `specrail mcp-server` starts a stdio JSON-RPC MCP server (`src/mcp.rs`).
- The server advertises the `io.modelcontextprotocol/ui` extension and exposes an interactive feature-navigate UI resource at `ui://specrail/feature-navigate`.
- `specrail_status` returns `structuredContent.workflow` with the recommended skill, blockers, next tools, and candidate feature/outcome for the TDD loop.
- The installable Copilot CLI plugin lives under `.github/plugin/`; `.github/plugin/.mcp.json` launches `specrail mcp-server` as the `specrail` MCP server, and `.github/plugin/skills/` contains workflow skills (`specrail-tdd`, `specrail-init`, `specrail-workflow`, `specrail-testing`, `specrail-activation`, `specrail-resume`).

## Practical pitfalls

- `.specrail/state/current.yaml` only tracks one active feature and one active outcome.
- Tests are registered in the manifest; they are not auto-discovered from the filesystem.
- `test add` records a path but does not verify that the file exists.
- `prerequisites`, `dependencies`, and `required_tests` are modeled but not meaningfully enforced yet; do not assume they drive execution.
- `ledger.jsonl` is append-only audit history, not the primary state store.

## Errors and work-arounds observed while onboarding

- Initial `cargo build` / `cargo test` runs can pause with `Blocking waiting for file lock on package cache` while Cargo acquires the shared cache lock. The work-around is simply to wait for the lock to clear and let the command continue; no code change is needed.
