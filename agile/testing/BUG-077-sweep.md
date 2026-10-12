<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-077 — Sweep

**Item:** [BUG-077](../items/BUG-077-a-farm-task-merged-in-merger-skips-the-farms-checks.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG077-01 | A Farm whose supplemental review is *Required*; a task checked and approved with no supplemental review yet | *Merge it in Merger* | Merger does not open; the notice says the task requires a current supplemental review | P1 | Not run. |
| SWEEP-BUG077-02 | As above, after the supplemental review passes | *Merge it in Merger*, make the merge | The task becomes done and its worktree goes | P1 | Not run. |
| SWEEP-BUG077-03 | A Farm with no supplemental review; a task checked and approved | *Merge it in Merger*, make the merge | As before FEAT-117's fix: done, worktree gone | P1 | Not run. |
