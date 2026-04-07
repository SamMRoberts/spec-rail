---
name: specrail-workflow
description: Use specrail MCP tools to manage the workflow state instead of editing .specrail files directly.
---

When a repository uses specrail, prefer the `specrail_*` MCP tools for reading and changing workflow state.

Use this sequence by default:

1. Call `specrail_status` to confirm whether the project is initialized and which feature or outcome is active.
2. Use the read tools (`specrail_feature_list`, `specrail_feature_show`, `specrail_outcome_list`, `specrail_outcome_show`, `specrail_test_list`, `specrail_trace`) before proposing changes.
3. Use the mutating tools to create or activate features, outcomes, and tests instead of writing `.specrail/*` files by hand.
4. After any mutating tool call, check `specrail_status` again to verify the new state.

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