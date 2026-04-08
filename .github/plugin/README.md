# Specrail Copilot Plugin

This plugin exposes the specrail MCP server and a set of workflow skills for driving a specrail project end to end.

In VS Code, the repo also ships a workspace custom agent at `.github/agents/specrail.agent.md`. Use that agent when you want repository-aware guidance that starts from the current SpecRail state and then leans on the MCP tools and the stage skills below. Use this plugin package when you want the installable Copilot CLI skill set.

## What It Covers

The plugin is organized around one top-level TDD entry point plus the detailed stage skills:

1. `specrail-tdd`: Start here for the full end-to-end test-driven workflow. It checks `specrail_status`, prompts for missing features and outcomes, follows the recommended next stage, and keeps looping until work is done.
2. `specrail-setup`: Bootstrap a new `.specrail/` project when the repository is not initialized.
3. `specrail-resume`: Inspect the current state, determine where work was left off, and confirm where to resume.
4. `specrail-plan-features`: Gather and structure features and outcomes through follow-up questions.
5. `specrail-prepare-tests`: Plan, register, generate, and update tests for each outcome.
6. `specrail-run-workflow`: Execute the workflow in order with `implement`, `verify`, and `advance`.

## Recommended Starting Point

- If the user wants broad end-to-end help, start with `specrail-tdd`.
- If the repository is not initialized, start with `specrail-setup`.
- If the repository is already in progress and the user wants to continue from the last stopping point, start with `specrail-resume`.
- If the repository is initialized but features and outcomes are not defined yet, start with `specrail-plan-features`.
- If features and outcomes exist but tests are incomplete, use `specrail-prepare-tests`.
- If tests are ready and the user wants to execute work in order, use `specrail-run-workflow`.

## Handoff Order

The normal progression is:

1. `specrail-tdd`
2. `specrail-setup`
3. `specrail-plan-features`
4. `specrail-prepare-tests`
5. `specrail-run-workflow`

If the project is already partway through the workflow, `specrail-tdd` should call `specrail_status`, use the `workflow.recommended_skill` guidance, and then hand off to `specrail-resume` or the matching stage instead of starting over.

Skills are the guided user-facing workflows. The `specrail_*` MCP tools are the direct executable actions that the skills call.
