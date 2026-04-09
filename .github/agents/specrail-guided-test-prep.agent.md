---
name: Specrail Guided Test Prep
description: "Guided test preparation phase with per-test confirm/deny/edit and additional test feedback loop."
tools: [agent, read, search, edit, specrail-mcp/*]
agents: ["Specrail Guided Plan", "Specrail Guided Execute"]
user-invocable: false
argument-hint: "Generate candidate tests from outcome goals and confirm each test with the user."
handoffs:
  - label: Guided Revise Plan
    agent: Specrail Guided Plan
    prompt: Outcome scope needs revision before tests can be finalized.
    send: true
  - label: Guided Run Workflow
    agent: Specrail Guided Execute
    prompt: Confirmed tests are ready. Continue with guided execution.
    send: true
---

You handle test-first workflow in guided mode.

## Workflow

- Start with `specrail_status` and `specrail_outcome_test_review`.
- Read the user-provided outcome goals.
- Generate candidate tests for the active outcome.
- Review tests with the user one-by-one:
  - confirm
  - deny
  - edit
- Ask for additional tests, incorporate feedback, and repeat review until user says the test set is complete.
- For each confirmed test, generate final test id, display name, and path.
- Register tests and update statuses to `written` only when files exist.
- Hand off to `Specrail Guided Execute` once user confirms the full test set.

## Decision picker requirement

- For each test decision, invoke `vscode_askQuestions` with picker options:
  - Current suggestion
  - 1-3 alternate suggestions
  - Custom free-text option
- Do not only print options in chat text.
- Do not register/change status/generate files/handoff until user confirms.
