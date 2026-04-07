# Copilot instructions for `specrail`

## What this repository is

`specrail` is a Rust CLI for a phase-gated, test-driven workflow. The binary is `specrail`, and nearly all behavior revolves around creating and updating files under `.specrail/`.

For a first pass, read these files in order:

1. `src/main.rs` - top-level command dispatch
2. `src/cli.rs` - CLI surface and arguments
3. `src/core/models.rs` - persisted domain model
4. `src/core/repository.rs` - canonical `.specrail` path resolver and YAML I/O
5. `src/commands/implement.rs`, `src/commands/verify.rs`, `src/commands/advance.rs` - main workflow loop
6. `tests/*.rs` - integration coverage and expected CLI behavior

## Repository layout

- `src/commands/`: command handlers
- `src/core/`: config, repository access, state, ledger, models
- `src/policy/`: phase and manifest validation rules
- `src/agents/`: adapter layer for `generic-shell`, `copilot`, and `codex`
- `src/prompts/`: prompt construction for `implement`
- `src/runtime/`: filesystem and process helpers
- `tests/`: integration tests using `assert_cmd`, `tempfile`, and `predicates`

Generated project state lives under `.specrail/`:

- `.specrail/project.yaml`
- `.specrail/features/*.yaml`
- `.specrail/phases/<feature>/*.yaml`
- `.specrail/tests/manifest.yaml`
- `.specrail/state/current.yaml`
- `.specrail/state/ledger.jsonl`

Treat `src/core/repository.rs` as the source of truth for where project files belong.

## Normal workflow

The intended CLI flow is:

1. `specrail init`
2. `specrail feature new ...`
3. `specrail phase new ...`
4. `specrail test add ...`
5. `specrail test set-status <id> written`
6. `specrail feature activate <id>`
7. `specrail phase activate <feature> <phase>`
8. `specrail implement`
9. `specrail verify`
10. `specrail advance`

Important gating rules:

- All commands except `init` require an existing `.specrail/` directory; discovery walks upward from the current directory.
- `implement` requires an active feature, an active phase, at least one registered test for that phase, and no phase tests left in `planned` status.
- `verify` uses `project.yaml:test_command` from the project root and marks the phase `verified` or `failed`.
- `advance` only works from a `verified` phase and activates the phase whose `order` is current `order + 1`.

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
- Allowed and forbidden paths are currently prompt/env hints, not enforced write protections.
- Shell execution uses `sh -c`, so agent and test commands assume a Unix-like shell environment.

## Practical pitfalls

- `.specrail/state/current.yaml` only tracks one active feature and one active phase.
- Tests are registered in the manifest; they are not auto-discovered from the filesystem.
- `test add` records a path but does not verify that the file exists.
- `prerequisites`, `dependencies`, and `required_tests` are modeled but not meaningfully enforced yet; do not assume they drive execution.
- `ledger.jsonl` is append-only audit history, not the primary state store.

## Errors and work-arounds observed while onboarding

- Initial `cargo build` / `cargo test` runs can pause with `Blocking waiting for file lock on package cache` while Cargo acquires the shared cache lock. The work-around is simply to wait for the lock to clear and let the command continue; no code change is needed.
- The repository did not already contain a `.github/` directory. Creating `.github/copilot-instructions.md` is safe and does not affect runtime behavior.
