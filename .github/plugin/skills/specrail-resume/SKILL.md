---
name: specrail-resume
description: Inspect the current specrail state, determine the most likely resume point, and ask the user to confirm where to resume before continuing.
---

Use this skill when the repository already has a `.specrail/` project and the user wants to continue work from wherever the workflow was left off.

When a repository uses specrail, prefer the `specrail_*` MCP tools to inspect current state before making changes.

## Default sequence

1. Call `specrail_status` to determine whether the project is initialized and to inspect the active feature, active outcome, feature summaries, and test counts.
2. If the project is not initialized, tell the user there is no workflow to resume yet and hand off to `specrail-init`.
3. Call `specrail_feature_navigate` (no arguments) to see a visual overview of all features with progress bars. Then call with `feature_id` to inspect the active feature's outcomes and test status.
4. Call `specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, `specrail_test_list`, and `specrail_trace` to gather the detailed state of features, outcomes, tests, and recent workflow history.
5. Determine the most likely resume point from the current active state first, then from incomplete work if no active state exists.
6. Summarize what was last in progress, what is already complete, and the recommended resume point.
7. Ask the user to confirm whether to resume from that point.
8. Tell the user exactly which feature and outcome the workflow would resume from, and which skill should be used next.
9. Do not activate, advance, or otherwise mutate workflow state until the user confirms the resume point.

## Resume status display

Present the project state as a clear summary:

```
🚂 Specrail — Resume Point
───────────────────────────
Active feature : auth — Authentication
Active outcome : login — User login flow
Outcome status : ⚡ Active

Features:
  ⚡ auth       — 1/3 outcomes verified
  ○  payment    — 0/2 outcomes verified  (not started)

Recommended: Resume auth → login
Next step  : specrail-testing (no written tests yet)
```

## Resume decision rules

- If there is an active feature and active outcome, prefer resuming there.
- If the active outcome is `failed`, recommend resuming that same outcome and fixing it before moving on.
- If the active outcome is `active`, recommend resuming that same outcome.
- If the active outcome is `pending` but its tests are still missing or `planned`, recommend resuming with `specrail-testing` for that outcome.
- If the active outcome is ready for implementation, recommend resuming with `specrail-activation` at that outcome.
- If there is no active outcome but there is an incomplete active feature, identify the next eligible outcome in that feature using outcome `order` and current status.
- If there is no active feature, choose the next incomplete feature and then the next incomplete outcome within it.
- Prefer the lowest ordered outcome whose status is not `verified` or `skipped`.
- Use `specrail_trace` to understand the last meaningful workflow event when state appears ambiguous.
- If the ledger and current state disagree, summarize the conflict and ask the user which source of truth to follow.

## Resume summary guidance

- Tell the user which features are complete and which are still in progress.
- Tell the user which outcome was last active or last failed.
- Tell the user whether testing, activation, or initialization is the correct next stage.
- Present the recommendation as a concrete resume point, such as "Resume Feature `calculator`, Outcome `addition`, starting with test preparation" or "Resume Feature `calculator`, Outcome `subtraction`, starting with implementation".

## User interaction guidance

- Ask for confirmation before mutating state.
- If the user wants a different resume point, summarize the tradeoff and let them choose explicitly.
- If the project was launched outside the repository root, pass the workspace path through the `cwd` argument.

## Example

- If `specrail_status` shows active feature `calculator` and active outcome `addition`, resume there first.
- If `addition` has tests still marked `planned`, recommend resuming with `specrail-testing`.
- If `addition` already has ready tests, recommend resuming with `specrail-activation`.
- If `addition` is verified and the next outcome by order is `subtraction`, recommend resuming from `subtraction`.

