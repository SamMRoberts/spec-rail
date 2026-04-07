# Specrail Skills

This directory contains the workflow skills used by the specrail plugin.

## Skill Index

### `specrail-init`
Use when the repository does not yet have a `.specrail/` project.

Responsibilities:
- Check whether specrail is already initialized.
- Run `specrail_init` when needed.
- Verify the initialized state.
- Hand off into planning, testing, and activation.

### `specrail-resume`
Use when the repository is already in progress and the user wants to continue from where work stopped.

Responsibilities:
- Inspect current feature, outcome, test, and ledger state.
- Determine the most likely resume point.
- Tell the user what is complete and what is still in progress.
- Ask the user to confirm where to resume before making changes.
- Hand off to testing or activation based on the recommended next step.

### `specrail-workflow`
Use when features and outcomes need to be discovered, clarified, and created.

Responsibilities:
- Ask follow-up questions.
- Gather all features and outcomes.
- Keep feature and outcome scope narrow.
- Confirm the structure before creating it.
- Hand off to testing and activation.

### `specrail-testing`
Use when outcomes need tests before implementation can proceed.

Responsibilities:
- Inspect the current test manifest.
- Ask for missing test scenarios.
- Register tests with `specrail_test_add`.
- Generate tests when requested.
- Move tests from `planned` to `written` when ready.

### `specrail-activation`
Use when the user wants to execute outcomes in the correct order.

Responsibilities:
- Choose feature order using dependencies.
- Choose outcome order using `order` and `prerequisites`.
- Activate the correct feature and outcome.
- Run `specrail_implement`, `specrail_verify`, and `specrail_advance` in sequence.
- Repeat until all outcomes and features are complete.

## Recommended Flow

1. Start with `specrail-init` if the project is not initialized.
2. Use `specrail-workflow` to gather and create features and outcomes.
3. Use `specrail-testing` to register and prepare tests.
4. Use `specrail-activation` to execute the workflow.

If the repository is already in progress, begin with `specrail-resume` or whichever skill matches the current state.
