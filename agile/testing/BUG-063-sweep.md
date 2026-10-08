<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-063 — Sweep

**Item:** [BUG-063](../items/BUG-063-a-conflicting-pull-request-is-not-marked.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG063-01 | `maxmya/trattoria-demo#3` conflicting with `main` | Open Review | Its card says *conflicts with main*; the preview says it cannot be merged as it stands | P1 | Not run. |
| SWEEP-BUG063-02 | As above | Open the review room | The header line says *Conflicts with main* | P1 | Not run. |
| SWEEP-BUG063-03 | As above | Open Pull requests, then #3 | The row and the workspace header say *conflicts* | P1 | Not run. |
| SWEEP-BUG063-04 | Workspace on #3, branch checked out locally | *Merge*, then *Resolve in Merger* | No merge is sent; Merger opens with A `main`, B the branch, *Into* B | P1 | Not run. |
| SWEEP-BUG063-05 | After resolving and pushing the branch | Refresh Review and Pull requests | The marks are gone | P2 | Not run. |
