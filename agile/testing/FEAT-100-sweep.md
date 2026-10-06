<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-100 — Manual sweep

**Item:** [`agile/items/FEAT-100-a-merger-that-shows-the-result-first.md`](../items/FEAT-100-a-merger-that-shows-the-result-first.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT100-01 | A repository with two branches that conflict | 1. Merger on the rail | Merger sits after Conflicts; the plan shows the checked-out branch as A, the newest other branch as B, *Dry run · nothing written yet* | P1 | |
| SWEEP-FEAT100-02 | As 01 | 1. Read the Result card | The sentence, three numbers, *N conflicts in M files* and the meter match what `git merge` would stop on | P1 | |
| SWEEP-FEAT100-03 | As 01 | 1. Into B 2. Swap 3. Into a new branch | Roles, arrows and colours follow; the new branch is named `merge/<a>-<b>`, editable | P1 | |
| SWEEP-FEAT100-04 | As 01 | 1. Each strategy | The sentence, numbers and History after change; Fast-forward only is disabled with its reason | P2 | |
| SWEEP-FEAT100-05 | As 01, `git status` clean | 1. Use the plan for a while 2. `git status`, `git worktree list` | Nothing changed, no worktree left | P1 | |
| SWEEP-FEAT100-06 | A remote branch as B | 1. Into B | Refused: a remote branch, merge into a new branch instead | P2 | |
| SWEEP-FEAT100-07 | Light and dark | 1. The plan in each | Cards, chips, arrows and meter read in both | P2 | |
| SWEEP-FEAT100-08 | Keyboard only | 1. Tab through the plan, open a picker, choose with the arrows | Every control is reachable and works | P2 | |
| SWEEP-FEAT100-09 | Git older than 2.38 (the linked dev machine, 2.34.1) | 1. Merger | The same forecast, from the scratch worktree; no worktree left | P1 | |
