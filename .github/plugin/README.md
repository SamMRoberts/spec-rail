# Specrail Copilot Plugin

This plugin exposes the specrail MCP server and a set of workflow skills for driving a specrail project end to end.

## What It Covers

The plugin is organized around five stages:

1. `specrail-init`: Bootstrap a new `.specrail/` project when the repository is not initialized.
2. `specrail-resume`: Inspect the current state, determine where work was left off, and confirm where to resume.
3. `specrail-workflow`: Gather and structure features and outcomes through follow-up questions.
4. `specrail-testing`: Plan, register, generate, and update tests for each outcome.
5. `specrail-activation`: Execute the workflow in order with `implement`, `verify`, and `advance`.

## Recommended Starting Point

- If the repository is not initialized, start with `specrail-init`.
- If the repository is already in progress and the user wants to continue from the last stopping point, start with `specrail-resume`.
- If the repository is initialized but features and outcomes are not defined yet, start with `specrail-workflow`.
- If features and outcomes exist but tests are incomplete, use `specrail-testing`.
- If tests are ready and the user wants to execute work in order, use `specrail-activation`.

## Handoff Order

The normal progression is:

1. `specrail-init`
2. `specrail-workflow`
3. `specrail-testing`
4. `specrail-activation`

If the project is already partway through the workflow, start with `specrail-resume` or start from the stage that matches the current state instead of starting over.
