<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-106 — Manual sweep

**Item:** [`agile/items/FEAT-106-a-rebase-you-can-see.md`](../items/FEAT-106-a-rebase-you-can-see.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT106-01 | A branch with an upstream, commits ahead | 1. Open Rebase | Planned onto the upstream; both cards, the result and the history drawn | P1 | |
| SWEEP-FEAT106-02 | As 01 | 1. Pick another starting point | Replanned at once | P1 | |
| SWEEP-FEAT106-03 | As 01 | 1. Squash, reword (type a message), drop, reorder 2. Rebase now | The result matches the history drawn; the reworded commit has the new message | P1 | |
| SWEEP-FEAT106-04 | A plan that conflicts | 1. Rebase now | Stopped in the Result card with Resolve / Continue / Skip / Abort; the plan locked | P1 | |
| SWEEP-FEAT106-05 | Light and dark | 1. Look | Matches Merger | P2 | |
