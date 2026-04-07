---
name: specrail-activation
description: Determine the best implementation order for specrail features and outcomes, then activate and advance through them in sequence until all are verified.
---

Use this skill when the user wants help deciding implementation order or wants to drive the workflow feature-by-feature and outcome-by-outcome.

When a repository uses specrail, prefer the `specrail_*` MCP tools to inspect state and move the workflow forward.

Default sequence:

1. Call `specrail_status` to identify the current active feature, current active outcome, and whether work is already in progress.
2. Call `specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, and `specrail_outcome_show` to gather the full set of features and outcomes before proposing an execution order.
3. Determine the best feature order using explicit dependencies first.
4. Within each feature, determine the best outcome order using `order` first and `prerequisites` second.
5. Activate the first eligible feature with `specrail_feature_activate`.
6. Activate the first eligible outcome in that feature with `specrail_outcome_activate`.
7. Keep the user focused on the active outcome until it is successfully verified.
8. After a successful outcome, move forward with `specrail_advance` or activate the next eligible outcome if needed.
9. When a feature has no remaining outcomes, move to the next eligible feature and repeat the same process.
10. Continue until all planned features and outcomes are verified or the user asks to stop.

Ordering rules:

- Prefer features whose dependencies are already complete.
- If feature dependencies are missing, ambiguous, or conflicting, ask follow-up questions before choosing an order.
- Prefer foundational features before dependent or polish-oriented features.
- Prefer incomplete features over already complete features.
- Within a feature, use the lowest `order` value first.
- If an outcome has prerequisites, do not activate it before its prerequisites are satisfied.
- If ordering metadata is incomplete, ask the user to confirm the intended sequence.

Execution rules:

- Do not switch away from an already active, unverified outcome unless the user explicitly asks to reorder or abandon it.
- Treat outcome success as `Verified`, not merely `Active`.
- Do not advance to the next outcome after activation alone.
- After each activation or advancement step, call `specrail_status` again to confirm the new state.
- If verification fails, keep the current outcome active and help the user resolve that outcome before moving on.
- Do not skip blocked or failed outcomes without explicit user approval.
- When a feature is complete, confirm that all of its outcomes are verified before moving to the next feature.

Recommended loop:

1. Identify the next eligible feature.
2. Activate that feature.
3. Identify the next eligible outcome in that feature.
4. Activate that outcome.
5. Work on that outcome until verification succeeds.
6. Advance to the next outcome.
7. Repeat until the feature is complete.
8. Move to the next feature.
9. Repeat until the full workflow is complete.

User interaction guidance:

- If the best implementation order is unclear, explain the tradeoffs and ask targeted follow-up questions.
- Summarize the proposed feature order and per-feature outcome order before making changes.
- Tell the user which feature and outcome are active after each successful transition.
- If the project was launched outside the repository root, pass the workspace path through the `cwd` argument.

Example:

- If Feature B depends on Feature A, activate Feature A first.
- If Feature A has outcomes with orders `1`, `2`, and `3`, activate them in that order.
- If outcome `2` fails verification, keep outcome `2` active and do not move to outcome `3` until outcome `2` is verified.
- Once all outcomes in Feature A are verified, move to the next eligible feature and repeat.
