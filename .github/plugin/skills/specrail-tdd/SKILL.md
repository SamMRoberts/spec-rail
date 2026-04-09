---
name: specrail-tdd
description: Start here for the full specrail test-driven workflow. Use specrail_status to find the next stage, then drive init, planning, testing, implementation, verification, and advancement in order.
---

Use this skill when the user wants broad help adopting specrail, wants to work end to end, or asks for a test-driven implementation workflow without naming a specific stage.

This is the umbrella, start-here skill. It should not silently hand off without first guiding the user into the next required conversation step.

## Status display

After every `specrail_status` call, show the user a concise project dashboard using this format:

```
🚂 Specrail Status
Active feature : <feature_id> – <title>   (or "none")
Active outcome : <outcome_id> – <title>   (or "none")
Recommended    : <workflow.recommended_skill>
Summary        : <workflow.summary>
```

If `workflow.blockers` is non-empty, list each blocker with a ⚠️ prefix.
If `workflow.next_tools` is available, list the next tools with a ▶ prefix.

## Default loop

1. Call `specrail_status` first. Display the project dashboard.
2. Read `structuredContent.workflow` from the response.
3. Use `workflow.recommended_skill`, `workflow.summary`, `workflow.blockers`, `workflow.blocker_details`, `workflow.actions`, and `workflow.next_tools` to choose the next stage.
4. If the recommended skill is `specrail-setup`, initialize the repository with `specrail_init` using `no_wizard: true` (the interactive wizard requires a live terminal and will block MCP execution), then call `specrail_status` again.
5. If the recommended skill is `specrail-plan-features`, begin an explicit feature and outcome interview before creating anything.
6. Ask the user for the first feature if none exists yet, or ask whether to keep or refine the next feature slice if features already exist.
7. Default to planning the next feature or outcome slice needed for the current workflow unless the user explicitly asks for the full roadmap.
8. Expand broad feature and outcome ideas into narrower, clearer slices, then confirm the expanded structure with the user before creating it.
9. Keep looping on features and outcomes until the user explicitly says they are done.
10. If the recommended skill is `specrail-prepare-tests`, prepare the tests for the current or next outcome until implementation is no longer blocked by missing or `planned` tests. Use `specrail_outcome_test_review` to inspect test gaps before asking questions.
11. If the recommended skill is `specrail-run-workflow`, run the canonical loop: activate the correct feature and outcome, `specrail_implement`, apply its returned delegation prompt in the current conversation, then `specrail_verify`, then `specrail_advance`.
12. After every mutating step, call `specrail_status` again and keep following the updated guidance until the workflow is complete or the user asks to stop.

When available, prefer `specrail_workflow_next` to restate the single best next action and `specrail_resume_point` when the user asks to continue from the latest stopping point.

## Feature and outcome interview rules

- Do not assume the user has already fully defined the workflow just because they asked for end-to-end help.
- Prompt for features and outcomes whenever they are missing, incomplete, or overly broad.
- Ask for one feature at a time, then ask for that feature's outcomes one by one.
- If the user gives a broad feature, help narrow it into a clearer product capability.
- If the user gives a broad outcome, expand it into narrower sibling outcomes under the same feature.
- Copilot should help expand rough ideas into a cleaner feature and outcome structure, but must confirm that structure with the user before creating it.
- Continue prompting only as far as needed to define the next clear implementation slice unless the user asks for more.
- After collecting the current slice, summarize the planned feature and outcome structure in a compact list before moving to testing.

## Clarification and scoping rules

- If the user appears to be starting from scratch, ask for any missing project context before planning features. At minimum, ask for platform, language, stack/framework, interface type, and deployment/runtime target.
- If the request is ambiguous, ask targeted follow-up questions instead of guessing which feature, outcome, or stage the user means.
- If the user references a feature, outcome, or test that may not exist, verify it first with `specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, and `specrail_test_list`, then ask whether to create it or correct the reference.
- If the request is too broad for a single feature or outcome, warn the user and ask whether to refine it into smaller slices before creating anything.
- Keep reminding the user that strong, well-defined tests are the foundation of the workflow and must be defined before code is written.

## Response formatting

- Use markdown tables to summarize feature and outcome lists:

```
| Feature | Status  | Outcomes | Verified |
|---------|---------|----------|----------|
| auth    | active  | 3        | 1/3      |
```

- Use status icons: ⚡ Active, ✅ Verified, ❌ Failed, ○ Pending
- After activation, always confirm the new state: "✅ Now working on: **auth** → **login**"

## Rules

- Prefer the MCP tools over editing `.specrail/*` files directly.
- **Test-first is non-negotiable**: never call `specrail_implement` until all required tests for the active outcome are registered, their files exist, and they are in `written` (not `planned`) status. Use `specrail_outcome_test_review` to verify this before implementation.
- Only create or generate tests that belong to the current outcome's required scope; justified regression tests are allowed when shared code is being touched, but say why.
- After `specrail_implement`, inspect `structuredContent.delegation` and carry out the returned implementation prompt yourself before moving on to verification.
- Once tests are defined and written, only implement the minimum code needed for those current tests to pass.
- Treat `workflow.blockers` as reasons to stop and resolve the blocking stage before running implementation.
- If the repository was opened outside the project root, pass the workspace path through `cwd`.
- When the user only asks for one stage, hand off to the more specific stage skill after the first `specrail_status` check.
- This skill is the umbrella guide; it should route the user into the more specific skills while relying on MCP tools for the actual state changes.

## Example

- If there are no features yet, ask the user for the first feature.
- After the user gives a feature such as `calculator`, ask for the outcomes under that feature.
- If the user gives a broad outcome like `calculator operations`, expand it into narrower outcomes such as `addition`, `subtraction`, `multiplication`, and `division`.
- Confirm the final feature and outcome structure with the user before creating it.
