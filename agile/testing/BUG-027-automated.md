<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-027 — Automated checks

Automatic merges require passing current checks and independent successful review, merge permission and the original destination. Failed or missing evidence prevents mutation. Invalid task states are rejected before Git runs; concurrent merges serialize and conflicts preserve work. Existing farms load with no trusted evidence until rechecked.

Regression tests exercise real disposable repositories and bounded child processes.
Record exact results and coverage below; no model credentials or user repositories
are needed for automated checks.

## Results

Headless Linux validation on 2026-09-05:

- Full Rust workspace: 959 tests passed before two additional core merge tests;
  both destination-race and concurrent-merge core tests also passed.
- Farm integration: 53 passing tests, including nine new evidence/permission/
  destination/restart/reviewer-failure regressions.
- Frontend farm tests: 112 passed; Svelte check: no errors or warnings.
- Workspace Clippy passed with warnings denied. Final formatting and record
  checks run before commit.

Review: automatic merge pins the checked commit, requires independent review,
checks the saved destination under the shared synchronous Git lock, and rejects
invalid states before mutation. External Git processes and asynchronous Git
operations still require their own coordination. UI and installer checks remain
in the manual sweep; they have not been performed.
