<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-047 — Manual sweep

**Item:** [`agile/items/TASK-047-a-commit-bar.md`](../items/TASK-047-a-commit-bar.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK047-01 | Staged changes | 1. Working copy | One bar at the bottom; the diff fills the column above it | P1 | |
| SWEEP-TASK047-02 | As -01 | 1. Type a summary 2. Commit | Commits as before | P1 | |
| SWEEP-TASK047-03 | As -01 | 1. Add description 2. Type 3. Leave and come back | Opens under the summary, focused; still open with the text when you return | P1 | |
| SWEEP-TASK047-04 | Signing on | 1. Look above the bar | The signing note, one line | P2 | |
| SWEEP-TASK047-05 | A clean working copy | 1. Working copy | "Nothing to commit", and no bar | P2 | |
