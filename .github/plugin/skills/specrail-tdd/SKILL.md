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
3. Use `workflow.recommended_skill`, `workflow.summary`, `workflow.blockers`, and `workflow.next_tools` to choose the next stage.
4. If the recommended skill is `specrail-setup`, initialize the repository and then call `specrail_status` again.
5. If the recommended skill is `specrail-plan-features`, begin an explicit feature and outcome interview before creating anything.
6. Ask the user for the first feature if none exists yet, or ask whether to keep or refine the existing feature list if features already exist.
7. For each feature, ask for the outcomes under that feature and keep asking until the user says that feature is complete.
8. Expand broad feature and outcome ideas into narrower, clearer slices, then confirm the expanded structure with the user before creating it.
9. Keep looping on features and outcomes until the user explicitly says they are done.
10. If the recommended skill is `specrail-prepare-tests`, prepare the tests for the current or next outcome until implementation is no longer blocked by missing or `planned` tests.
11. If the recommended skill is `specrail-run-workflow`, run the canonical loop: activate the correct feature and outcome, `specrail_implement`, `specrail_verify`, then `specrail_advance`.
12. After every mutating step, call `specrail_status` again and keep following the updated guidance until the workflow is complete or the user asks to stop.

## Feature and outcome interview rules

- Do not assume the user has already fully defined the workflow just because they asked for end-to-end help.
- Prompt for features and outcomes whenever they are missing, incomplete, or overly broad.
- Ask for one feature at a time, then ask for that feature's outcomes one by one.
- If the user gives a broad feature, help narrow it into a clearer product capability.
- If the user gives a broad outcome, expand it into narrower sibling outcomes under the same feature.
- Copilot should help expand rough ideas into a cleaner feature and outcome structure, but must confirm that structure with the user before creating it.
- Continue prompting until the user explicitly says there are no more features or outcomes to add.
- After collecting the structure, summarize the planned features and outcomes in a compact list before moving to testing.

## Response formatting

- Use `specrail_feature_navigate` to show the interactive feature/outcome picker UI whenever the user asks to browse features or outcomes.
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
- Keep the user in a test-first flow: define outcomes, define tests, move tests to `written`, then implement.
- Only create or generate tests that belong to the current outcome's required scope; do not pad the suite with speculative tests for future outcomes.
- Once tests are defined, only implement the minimum code needed for those current tests to pass.
- Treat `workflow.blockers` as reasons to stop and resolve the blocking stage before running implementation.
- If the repository was opened outside the project root, pass the workspace path through `cwd`.
- When the user only asks for one stage, hand off to the more specific stage skill after the first `specrail_status` check.
- This skill is the umbrella guide; it should route the user into the more specific skills while relying on MCP tools for the actual state changes.

## Example

- If there are no features yet, ask the user for the first feature.
- After the user gives a feature such as `calculator`, ask for the outcomes under that feature.
- If the user gives a broad outcome like `calculator operations`, expand it into narrower outcomes such as `addition`, `subtraction`, `multiplication`, and `division`.
- Confirm the final feature and outcome structure with the user before creating it.
