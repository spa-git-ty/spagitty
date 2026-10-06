<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-101 — Manual sweep

**Item:** [`agile/items/FEAT-101-merge-without-conflicts.md`](../items/FEAT-101-merge-without-conflicts.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT101-01 | Two branches that merge cleanly, A checked out | 1. Merger, Into A, Merge commit 2. Merge now 3. Create merge commit | A has the merge commit with both parents; its files updated; done says *A now includes B* | P1 | |
| SWEEP-FEAT101-02 | As 01, with an unrelated uncommitted change in A's worktree | 1. Into B, Merge now, Create merge commit | B has the merge; A, its files and the uncommitted change are untouched; `git status` unchanged | P1 | |
| SWEEP-FEAT101-03 | As 01 | 1. Squash, Merge now 2. Edit the message 3. Commit the squash | One commit on the target with the typed message | P1 | |
| SWEEP-FEAT101-04 | As 01 | 1. Rebase, then fast-forward 2. Finish the rebase | The target is a straight line ending in the replayed commits; the source did not move; no worktree left | P1 | |
| SWEEP-FEAT101-05 | One branch behind the other | 1. Fast-forward only into the one behind | It moves forward; no commit is written | P2 | |
| SWEEP-FEAT101-06 | As 01 | 1. Into a new branch, Merge now, Create merge commit | The new branch exists at the merge; A and B unmoved | P2 | |
| SWEEP-FEAT101-07 | As 01 | 1. Merge now 2. Commit on B from a terminal 3. Create merge commit | Refused: B changed since it was read; nothing moved | P2 | |
| SWEEP-FEAT101-08 | As 01 | 1. Done 2. Open in Graph | The graph shows the merge | P3 | |
