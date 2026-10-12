<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-117 — Sweep

**Item:** [FEAT-117](../items/FEAT-117-every-merge-happens-in-merger.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT117-01 | A pull request that merges cleanly | Pull requests › the pull request › *Open in Merger*, choose *Squash*, *Merge pull request* | It is merged through the host with a squash; the screen says so; Pull requests then lists the others | P1 | Not run. |
| SWEEP-FEAT117-02 | A pull request that conflicts with its target | Open it in Merger, *Resolve the conflicts here*, resolve, merge, *Push*, *Back to the pull request*, *Merge pull request* | The branch is pushed and the host merges it | P1 | Not run. |
| SWEEP-FEAT117-03 | Graph with another branch checked out | Right-click a branch label › *Merge into <current> in Merger* | Merger opens with that pair and direction | P1 | Not run. |
| SWEEP-FEAT117-04 | A Farm task in review | *Merge it in Merger*, make the merge | The task becomes done and its worktree goes | P1 | Not run. |
