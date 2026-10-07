<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-104 — Automated test record

**Item:** [`agile/items/FEAT-104-a-branch-named-where-head-is.md`](../items/FEAT-104-a-branch-named-where-head-is.md)

| Suite | Cases |
| --- | --- |
| `src/lib/graph/branching.test.ts` | Names are trimmed with spaces as dashes; one commit at a time. |
| `src/lib/graph/naming.test.ts` | The field opens focused in the asked row only; Enter creates the dashed name there and closes; Escape creates nothing; an empty name does nothing. |
| `src/lib/graph/actions.test.ts` | `createBranchNamed` creates and checks out without a prompt. |
| `src/lib/chrome/chrome.test.ts` | Branch opens the field at HEAD on the graph, not the Branches screen. |
