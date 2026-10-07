<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-106 — Automated test record

**Item:** [`agile/items/FEAT-106-a-rebase-you-can-see.md`](../items/FEAT-106-a-rebase-you-can-see.md)

| Suite | Cases |
| --- | --- |
| `src/lib/rebase/plan.test.ts` | Counts per edit and the result before the preview; the sentence, and the one for an emptied branch; a squash with nothing above it. |
| `src/lib/rebase/panes.test.ts` | Plan rows, actions and their hints, keyboard and drag reordering (ported from TodoList); the reword message kept across a change of mind; squash and conflict tags; locked while git runs. History: the starting point and each commit, folded counts and risk rings, the oldest summed past eight, the dropped listed. |
| `src/routes/rebase/page.test.ts` | Asks for a starting point; picking one plans; result, plan and history, then Rebase now; conflict counts; a refused plan; nothing to replay; a read failure; progress; the ways on when stopped; truncation; a detached HEAD. |

Seen in a dev preview (headless Chrome, dark and light) with a seeded six-commit plan.
