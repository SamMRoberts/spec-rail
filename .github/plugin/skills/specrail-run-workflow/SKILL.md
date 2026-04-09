---
name: specrail-run-workflow
description: Run the specrail workflow in the right order until outcomes are verified.
---

Use this skill when the user wants help deciding implementation order or wants to drive the workflow feature-by-feature and outcome-by-outcome.

When a repository uses specrail, prefer the `specrail_*` MCP tools to inspect state and move the workflow forward.

If the project is not initialized yet, use the `specrail-setup` skill or call `specrail_init` before applying this workflow.

If the active or planned outcomes do not yet have registered tests, existing test files, and non-`planned` status, use the `specrail-prepare-tests` skill before running implementation.

## Default sequence

1. Call `specrail_status` to identify the current active feature, current active outcome, and whether work is already in progress.
   - Prefer `workflow.actions` over reconstructing tool order from `next_tools` alone.
2. Call `specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, and `specrail_test_list` to gather the full set of features, outcomes, and registered tests before proposing an execution order.
3. Determine the best feature order using explicit dependencies first.
4. Within each feature, determine the best outcome order using `order` first and `prerequisites` second.
5. Before activating or implementing an outcome, call `specrail_outcome_test_review` to verify that the outcome has registered tests, existing test files, and no missing or `planned` required coverage.
6. If tests are missing, files do not exist, or tests are still `planned`, use `specrail_test_add`, `specrail_test_generate`, and `specrail_test_set_status`, or hand off to the `specrail-prepare-tests` skill, before continuing.
7. Activate the first eligible feature with `specrail_feature_activate`.
8. Activate the first eligible outcome in that feature with `specrail_outcome_activate`.
   - If `specrail_status` already identifies a ready candidate, prefer `specrail_activate_next`.
9. Run `specrail_implement` for the active outcome.
10. Read `structuredContent.delegation`, apply `delegation.prompt` yourself in the current conversation, and keep edits within `allowed_paths` and `forbidden_paths`.
11. Run `specrail_verify` for the active outcome.
12. If verification succeeds, run `specrail_advance` to move to the next outcome.
13. When a feature has no remaining outcomes, move to the next eligible feature and repeat the same process.
14. Continue until all planned features and outcomes are verified or the user asks to stop.

## Ordering rules

- Prefer features whose dependencies are already complete.
- If feature dependencies are missing, ambiguous, or conflicting, ask follow-up questions before choosing an order.
- Prefer foundational features before dependent or polish-oriented features.
- Prefer incomplete features over already complete features.
- Within a feature, use the lowest `order` value first.
- If an outcome has prerequisites, do not activate it before its prerequisites are satisfied.
- If ordering metadata is incomplete, ask the user to confirm the intended sequence.

## Execution rules

- Do not switch away from an already active, unverified outcome unless the user explicitly asks to reorder or abandon it.
- Treat outcome success as `Verified`, not merely `Active`.
- Do not advance to the next outcome after activation alone.
- Follow the canonical execution loop: `specrail_implement`, apply the returned delegation prompt in the current conversation, then `specrail_verify`, then `specrail_advance`.
- Before `specrail_implement`, call `specrail_outcome_test_review` and constrain the scope to the active outcome's declared tests plus any explicitly justified regression coverage.
- During implementation, write only the smallest amount of code needed to satisfy the active outcome's tests.
- Treat `specrail_implement` as prompt preparation only; do not assume it already changed workspace files.
- Do not implement future outcomes, speculative abstractions, or extra behavior that is not required by the current tests.
- Do not bypass `specrail_advance` by directly activating the next outcome unless the user explicitly wants a manual override.
- After each activation or advancement step, call `specrail_status` again to confirm the new state.
- If the user asks what to do next, prefer `specrail_workflow_next`.
- After each implementation or verification step, inspect the result before moving on.
- If verification fails, keep the current outcome active and help the user resolve that outcome before moving on.
- Do not skip blocked or failed outcomes without explicit user approval.
- When a feature is complete, confirm that all of its outcomes are verified before moving to the next feature.

## Progress display

After each `specrail_advance` call, show progress:

```
✅ Verified: auth → login
▶  Next up : auth → register (Outcome 2)
   Progress: auth — 1/3 outcomes complete (33%)
```

When a feature is completed:
```
🎉 Feature complete: auth — all 3 outcomes verified!
▶  Moving to next feature: payment
```

## Recommended loop

1. Identify the next eligible feature.
2. Activate that feature.
3. Identify the next eligible outcome in that feature.
4. Call `specrail_outcome_test_review` to confirm that the outcome's tests are registered, their files exist, they are non-`planned`, and they have no gaps.
5. If test gaps exist, resolve them with `specrail_test_add`, `specrail_test_generate`, and `specrail_test_set_status` before proceeding.
6. Optionally run an explicit red phase once tests are written so the current test command still proves the new work fails before implementation.
7. Activate that outcome.
8. Run `specrail_implement`.
9. Apply `structuredContent.delegation.prompt` in the current conversation while respecting the returned path constraints.
10. Run `specrail_verify`.
11. If verification succeeds, run `specrail_advance`.
12. Repeat until the feature is complete.
13. Move to the next feature.
14. Repeat until the full workflow is complete.

## User interaction guidance

- If the best implementation order is unclear, explain the tradeoffs and ask targeted follow-up questions.
- Summarize the proposed feature order and per-feature outcome order before making changes.
- Before implementation, remind the user which tests define the current scope and that no extra code should be added beyond what those tests require.
- Tell the user which feature and outcome are active after each successful transition.
- If the project was launched outside the repository root, pass the workspace path through the `cwd` argument.
- This skill guides execution order and orchestration; the `specrail_*activate`, `specrail_verify`, and `specrail_advance` tools perform direct state changes, while `specrail_implement` returns the delegated implementation prompt for the parent agent to execute.

## Example

- If Feature B depends on Feature A, activate Feature A first.
- If Feature A has outcomes with orders `1`, `2`, and `3`, activate them in that order.
- If outcome `2` still has tests in `planned`, stop and use the testing workflow before running `specrail_implement`.
- If outcome `2` fails verification, keep outcome `2` active and do not move to outcome `3` until outcome `2` is verified.
- Once all outcomes in Feature A are verified, move to the next eligible feature and repeat.
