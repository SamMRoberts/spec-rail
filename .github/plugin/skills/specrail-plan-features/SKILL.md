---
name: specrail-plan-features
description: Plan and create specrail features and outcomes using MCP tools instead of editing .specrail files directly.
---

When a repository uses specrail, prefer the `specrail_*` MCP tools for reading and changing workflow state.

If `specrail_status` shows that the project is not initialized yet, start with the `specrail-setup` skill or call `specrail_init` before gathering features and outcomes.

## Use the navigator UI

Call `specrail_feature_navigate` (without arguments) whenever the user wants to see all features or needs to pick one. This tool returns an interactive panel that shows:
- Feature cards with progress bars (verified outcomes / total)
- Status badges: ⚡ Active, ✅ Verified, ❌ Failed, ○ Pending
- Per-outcome test counts and test-readiness warnings

When a feature is selected (by passing `feature_id`), the navigator shows outcomes with:
- Test counts (total / passing / planned)
- Outcome status and activation controls

## Default sequence

1. Call `specrail_status` to confirm whether the project is initialized and which feature or outcome is active.
2. Use the read tools (`specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, `specrail_test_list`, `specrail_trace`) before proposing changes.
3. Ask follow-up questions to gather the next feature or outcome slice needed right now unless the user explicitly asks for a full roadmap.
4. Keep the conversation incremental by default: define the next feature or the next outcome under the current feature, then pause for confirmation before expanding further.
5. Summarize the collected features and outcomes back to the user so they can confirm the structure before creation.
6. Use the mutating tools to create or activate features, outcomes, and tests instead of writing `.specrail/*` files by hand.
7. After any mutating tool call, check `specrail_status` again to verify the new state.
8. After defining the features and outcomes, hand off to the `specrail-prepare-tests` skill to register the required tests for each outcome before implementation begins.
9. Once tests are ready, hand off to the `specrail-run-workflow` skill to drive the `implement`, `verify`, and `advance` loop.

## Discovery behavior

- Do not assume the full workflow structure from a brief request.
- Ask targeted follow-up questions when features, outcomes, or scope boundaries are unclear.
- Default to planning the next slice, not the whole roadmap.
- Only gather all planned features up front when the user asks for a full roadmap or when cross-feature ordering truly depends on it.
- For each feature, continue asking for additional outcomes only while that helps define the next implementation slice.
- After finishing one feature, ask whether there is another feature to add only if the user wants to keep planning.
- If the user gives a broad feature, help break it into smaller, outcome-sized slices.
- If the user gives a broad outcome, ask how to split it into narrower sibling outcomes under the same feature.
- Before creating anything, restate the current feature and outcome list in a compact structure for confirmation.

## Validation and ambiguity resolution

- If the user is starting from scratch, gather missing project context before locking in features and outcomes. Ask for platform, language, stack/framework, interface type, and deployment/runtime target when those details are missing.
- If the request is ambiguous, ask clarifying questions before proposing a feature or outcome structure.
- If the user references a feature, outcome, or test that does not exist yet, verify that with `specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, and `specrail_test_list`, then ask whether to create it or correct the reference.
- If the request is too broad, say so explicitly and ask whether to continue as-is or refine it into narrower features or outcomes first.
- Keep the conversation anchored in TDD: the feature and outcome structure should lead to clear required tests, and tests must be defined before implementation begins.

## Workflow scope rules

- Each feature should describe one clear product capability.
- Each outcome should describe one clear behavior or slice of that feature.
- Do not bundle multiple distinct behaviors into a single outcome.
- If a feature has several behaviors, create multiple sibling outcomes under the same parent feature.

## Response formatting

When presenting features and outcomes to confirm, use a compact structured list:

```
Features to create:
  • calculator — Basic arithmetic operations
    Outcomes:
      1. addition — Verify 2 + 2 = 4
      2. subtraction — Verify 2 - 1 = 1
      3. multiplication — Verify 3 × 4 = 12
```

After each creation, confirm with: "✓ Created feature **calculator** with 3 outcomes."

## Example

- For a calculator feature, create separate outcomes for addition, subtraction, multiplication, and division.
- Do not create one broad outcome like "calculator operations" that mixes all operations together.
- For the addition outcome, keep the scope focused on addition-specific behavior such as `2 + 2 = 4`, `5 + 0 = 5`, and `0 + 0 = 0`.
- For the subtraction outcome, keep the scope focused on subtraction-specific behavior such as `2 - 2 = 0`, `2 - 0 = 2`, and `0 - 2 = -2`.

If the MCP server was not launched from the project root, pass the workspace path through the `cwd` argument.

This skill is the planning conversation; the `specrail_*` tools perform the actual create and activate actions.
