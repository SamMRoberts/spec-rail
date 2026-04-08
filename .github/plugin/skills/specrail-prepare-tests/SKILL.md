---
name: specrail-prepare-tests
description: Prepare tests for specrail outcomes so implementation is not blocked by missing or planned tests.
---

Use this skill when the user wants to define tests for features or outcomes, when an outcome has no registered tests, or when implementation is blocked because tests are still `planned`.

When a repository uses specrail, prefer the `specrail_*` MCP tools to inspect and update the test manifest.

If the project is not initialized yet, use the `specrail-setup` skill or call `specrail_init` before applying this workflow.

Default sequence:

1. Call `specrail_status` to confirm whether the project is initialized and which feature and outcome are active.
2. Call `specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, and `specrail_test_list` to gather the current feature, outcome, and test state.
3. Ask follow-up questions to determine which tests are required for each outcome if the user has not already provided them clearly.
4. Work outcome-by-outcome and gather the expected unit, integration, or e2e tests for that outcome.
5. For each missing test, register it with `specrail_test_add`.
6. If the user wants help creating the test files, use `specrail_test_generate`.
7. Once a test exists and is ready to be executed, update it from `planned` to `written` with `specrail_test_set_status`.
8. After verification runs, update test statuses to reflect the latest known state when needed.
9. Re-check `specrail_test_list` and `specrail_status` to confirm that the outcome is no longer blocked by missing or `planned` tests.

Test planning rules:

- Do not assume tests can be skipped just because the outcome sounds simple.
- At least one registered test should exist for each outcome before implementation.
- Gather tests narrowly so each test clearly supports the current outcome.
- Prefer asking for concrete examples, edge cases, and failure cases.
- If the user gives broad test ideas, split them into distinct test cases.
- Keep test IDs and paths stable and specific to the feature and outcome they support.

Status rules:

- Use `planned` only for tests that have been identified but not yet written.
- Use `written` once the test exists and is ready to run.
- Use `passing` or `failing` only to reflect known execution results, not guesses.
- Do not leave required outcome tests in `planned` if the next step is implementation.

Recommended loop:

1. Identify the current feature and outcome.
2. Inspect existing tests for that outcome.
3. Ask for any missing test scenarios.
4. Register each missing test.
5. Generate or write tests if requested.
6. Move ready tests to `written`.
7. Repeat for the next outcome.
8. Hand off to `specrail-run-workflow` once tests are ready.

User interaction guidance:

- Ask follow-up questions until the test set for the current outcome is complete.
- Summarize the planned tests for each outcome before registering them.
- Tell the user when an outcome still has `planned` tests that block implementation.
- If the project was launched outside the repository root, pass the workspace path through the `cwd` argument.
- This skill guides test preparation; the `specrail_test_*` MCP tools perform the direct test registration and status updates.

Example:

- For an addition outcome, gather tests such as `2 + 2 = 4`, `5 + 0 = 5`, and `0 + 0 = 0`.
- Register each test with `specrail_test_add`.
- If the test files have been generated or written, move them to `written` with `specrail_test_set_status`.
- Only then proceed into the implementation workflow.
