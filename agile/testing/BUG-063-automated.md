<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-063 — Automated test record

**Item:** [BUG-063](../items/BUG-063-a-conflicting-pull-request-is-not-marked.md)

| Suite | Cases |
| --- | --- |
| `src/lib/review/inbox.test.ts` | A pull request the host cannot merge gets the *conflicts with <base>* chip and the first fact, on both the needs-you and back-with-you groups; one the host has not decided, or can merge, gets neither. |
| `src/routes/review/page.test.ts`, `room.test.ts` | The card and the preview say it conflicts; the room's header says *Conflicts with main*, and says nothing while the host has not said. |
| `src/lib/requests/requests.test.ts`, `workspace.test.ts` | The list row and the workspace header carry *conflicts* for a pull request the host cannot merge and nothing for an undecided one. |

## Results — 2026-10-11

- `bunx vitest run`: 3,809 passed. `bun run check`: zero errors, zero warnings.
