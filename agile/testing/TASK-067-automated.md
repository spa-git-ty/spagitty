<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-067 — Automated test record

**Item:** [TASK-067](../items/TASK-067-the-attribution-check-does-not-read-who-wrote-a-commit.md)

- `tools/attribution.test.ts`, *names a commit whose author or committer is an agent*: an agent author and an agent committer are each named with the commit's short hash.
- `tools/attribution.test.ts`, *lets a person who shares a name with an agent write a commit (BUG-078)*: Claude Dupont, Devin Smith and GitHub's own committer pass.

## Results — 2026-10-19

- `bunx vitest run tools/attribution.test.ts`: 10 passed.
- `node tools/attribution.mjs commits origin/main..HEAD` on `release/3.4.0`: exit 0.
- On `48bc351~30..48bc351`, history already on `main`, it names `f475d8b author Antigravity <antigravity@google.com>` beside the trailers it found before. That range is history and is never read by gate 1; it was run only to show the check finds what the item says it missed.
