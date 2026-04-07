---
name: specrail-init
description: Initialize a specrail project, verify the starting state, and hand off into feature, outcome, and test planning.
---

Use this skill when the repository does not yet have a `.specrail/` project, when `specrail_status` shows the project is not initialized, or when the user wants to bootstrap specrail in a new workspace.

When a repository uses specrail, prefer the `specrail_*` MCP tools to create and inspect the initial workflow state.

Default sequence:

1. Call `specrail_status` to determine whether the current directory is already initialized.
2. If the project is already initialized, do not run `specrail_init` again unless the user explicitly wants to refresh missing files.
3. If the project is not initialized, call `specrail_init` with the workspace `cwd`.
4. Call `specrail_status` again to confirm that initialization succeeded.
5. Explain that initialization creates the base `.specrail/` structure, manifest, state file, and ledger.
6. If the user wants to define the workflow immediately, hand off to the `specrail-workflow` skill to gather features and outcomes.
7. After features and outcomes are defined, hand off to the `specrail-testing` skill to register required tests.
8. Once tests are ready, hand off to the `specrail-activation` skill to execute the `implement`, `verify`, and `advance` loop.

Initialization rules:

- Treat initialization as repository bootstrap, not as a substitute for feature and outcome planning.
- Confirm the working directory before initializing if the repository root is unclear.
- Prefer `specrail_init` over manually creating `.specrail/*` files.
- After initialization, verify the resulting state before moving on.
- If the user wants a custom workflow structure, gather that through follow-up questions after init rather than trying to encode it into init itself.

User interaction guidance:

- Tell the user whether specrail was newly initialized or was already present.
- If the repository is already initialized, summarize the current state instead of re-bootstrapping it.
- If the project was launched outside the repository root, pass the workspace path through the `cwd` argument.

Example:

- If `specrail_status` shows no initialized project, run `specrail_init`.
- Confirm the project is now initialized.
- Then move into workflow planning, test planning, and activation in that order.
