---
name: specrail-tdd
description: Start here for the full specrail test-driven workflow. Use specrail_status to find the next stage, then drive init, planning, testing, implementation, verification, and advancement in order.
---

Use this skill when the user wants broad help adopting specrail, wants to work end to end, or asks for a test-driven implementation workflow without naming a specific stage.

Default loop:

1. Call `specrail_status` first.
2. Read `structuredContent.workflow` from the response.
3. Use `workflow.recommended_skill`, `workflow.summary`, `workflow.blockers`, and `workflow.next_tools` to choose the next stage.
4. If the recommended skill is `specrail-init`, initialize the repository and then call `specrail_status` again.
5. If the recommended skill is `specrail-workflow`, gather or confirm the feature and outcome structure before creating anything.
6. If the recommended skill is `specrail-testing`, prepare the tests for the current or next outcome until implementation is no longer blocked by missing or `planned` tests.
7. If the recommended skill is `specrail-activation`, run the canonical loop: activate the correct feature and outcome, `specrail_implement`, `specrail_verify`, then `specrail_advance`.
8. After every mutating step, call `specrail_status` again and keep following the updated guidance until the workflow is complete or the user asks to stop.

Rules:

- Prefer the MCP tools over editing `.specrail/*` files directly.
- Keep the user in a test-first flow: define outcomes, define tests, move tests to `written`, then implement.
- Treat `workflow.blockers` as reasons to stop and resolve the blocking stage before running implementation.
- If the repository was opened outside the project root, pass the workspace path through `cwd`.
- When the user only asks for one stage, hand off to the more specific stage skill after the first `specrail_status` check.
