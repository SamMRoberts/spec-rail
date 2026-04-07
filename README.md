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
├── features/
├── phases/
├── tests/manifest.yaml
└── state/
    ├── current.yaml
    └── ledger.jsonl
```

Key rules:

- `specrail init` creates the project structure.
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

## Development notes

- The binary name is `specrail`.
- The default shell-based agent assumes a Unix-like `sh -c` environment.
- Integration tests in `tests/` demonstrate expected CLI behavior and end-to-end workflow.
