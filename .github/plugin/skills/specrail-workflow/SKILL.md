---
name: specrail-workflow
description: Use specrail MCP tools to manage the workflow state instead of editing .specrail files directly.
---

When a repository uses specrail, prefer the `specrail_*` MCP tools for reading and changing workflow state.

If `specrail_status` shows that the project is not initialized yet, start with the `specrail-init` skill or call `specrail_init` before gathering features and outcomes.

Use this sequence by default:

1. Call `specrail_status` to confirm whether the project is initialized and which feature or outcome is active.
2. Use the read tools (`specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, `specrail_test_list`, `specrail_trace`) before proposing changes.
3. Ask follow-up questions to gather the full set of features and outcomes when the user has not already provided them completely.
4. Keep asking for the next feature or the next outcome under the current feature until the user explicitly says they are done.
5. Summarize the collected features and outcomes back to the user so they can confirm the structure before creation.
6. Use the mutating tools to create or activate features, outcomes, and tests instead of writing `.specrail/*` files by hand.
7. After any mutating tool call, check `specrail_status` again to verify the new state.
8. After defining the features and outcomes, hand off to the `specrail-testing` skill to register the required tests for each outcome before implementation begins.
9. Once tests are ready, hand off to the `specrail-activation` skill to drive the `implement`, `verify`, and `advance` loop.

Discovery behavior:

- Do not assume the full workflow structure from a brief request.
- Ask targeted follow-up questions when features, outcomes, or scope boundaries are unclear.
- Gather all planned features first, or work feature-by-feature if that is easier for the user.
- For each feature, continue asking for additional outcomes until the user says that feature is complete.
- After finishing one feature, ask whether there is another feature to add.
- Continue this loop until the user explicitly says they are done adding features and outcomes.
- If the user gives a broad feature, help break it into smaller, outcome-sized slices.
- If the user gives a broad outcome, ask how to split it into narrower sibling outcomes under the same feature.
- Before creating anything, restate the current feature and outcome list in a compact structure for confirmation.

Model workflow scope narrowly:

- Each feature should describe one clear product capability.
- Each outcome should describe one clear behavior or slice of that feature.
- Do not bundle multiple distinct behaviors into a single outcome.
- If a feature has several behaviors, create multiple sibling outcomes under the same parent feature.

Example:

- For a calculator feature, create separate outcomes for addition, subtraction, multiplication, and division.
- Do not create one broad outcome like "calculator operations" that mixes all operations together.
- For the addition outcome, keep the scope focused on addition-specific behavior such as `2 + 2 = 4`, `5 + 0 = 5`, and `0 + 0 = 0`.
- For the subtraction outcome, keep the scope focused on subtraction-specific behavior such as `2 - 2 = 0`, `2 - 0 = 2`, and `0 - 2 = -2`.

If the MCP server was not launched from the project root, pass the workspace path through the `cwd` argument.