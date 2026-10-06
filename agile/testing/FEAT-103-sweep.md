<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-103 — Manual sweep

**Item:** [`agile/items/FEAT-103-a-rebase-that-stops-in-merger.md`](../items/FEAT-103-a-rebase-that-stops-in-merger.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT103-01 | B has three commits, the first conflicting with A | 1. Merger, Rebase, then fast-forward 2. Resolve | *Rebasing B onto A · commit 1 of 3*, the commit named, its conflicts in the columns; A, B and `git status` unchanged | P1 | |
| SWEEP-FEAT103-02 | As 01 | 1. Resolve 2. Continue | The next stop or the dialog; Finish the rebase moves A; B is unmoved; each commit keeps its message | P1 | |
| SWEEP-FEAT103-03 | As 01, stopped | 1. Leave for Graph 2. Back, Resolve | The same stop | P1 | |
| SWEEP-FEAT103-04 | As 01, stopped | 1. Skip this commit | The commit is dropped; the rebase goes on | P2 | |
| SWEEP-FEAT103-05 | As 01, stopped | 1. Abort, confirm 2. `git worktree list`, `git status` | Back to the plan; no worktree left; nothing changed | P1 | |
| SWEEP-FEAT103-06 | As 01 | 1. Into B 2. Resolve | The columns still read A, then B | P2 | |
| SWEEP-FEAT103-07 | Two branches with nothing in each other's way | 1. Rebase, Merge now | Straight to the dialog; Finish the rebase lands it | P2 | |
