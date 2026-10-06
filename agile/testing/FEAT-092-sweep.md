<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-092 — Manual sweep

**Item:** [`agile/items/FEAT-092-conflict-fix-origin.md`](../items/FEAT-092-conflict-fix-origin.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT092-01 | — | 1. The room with the handoff's sample, conflict fixes answered by fixtures 2. types.ts 3. main's side | A sky card headed as a conflict fix, the merge named, sky markers on its lines, main's lines under the header | P1 | Pass, 2026-10-04, dev server in headless Chrome |
| SWEEP-FEAT092-02 | A GitHub pull request that merged main and resolved a conflict | 1. Open it in Review | The resolution is a sky card; the file says *conflict fix*; each side opens | P1 | |
| SWEEP-FEAT092-03 | The same on the author's work GitLab | 1. As 02 | The same | P1 | |
| SWEEP-FEAT092-04 | As 02 | 1. Author 2. Conflict fixes | A file changed only by the resolution is under Conflict fixes and not Author | P2 | |
| SWEEP-FEAT092-05 | As 02, then back to the inbox | 1. Look at the pull request's card | It says *conflict fixes* | P2 | |
| SWEEP-FEAT092-06 | A pull request with no merges | 1. Open it | No sky cards; the legend alone | P2 | |
