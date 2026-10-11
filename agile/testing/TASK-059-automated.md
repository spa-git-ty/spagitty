<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-059 — Automated test record

**Item:** [TASK-059](../items/TASK-059-reading-font-and-ruler-off-by-default.md)

| Suite | Cases |
| --- | --- |
| `src/lib/reading.test.ts` | The defaults have the ruler off and the review room off; the Reading section offers *Use the reading set*. |
| `src/routes/review/room.test.ts` | The room opens with no ruler and plain code and has no *Focus ruler* or *Reading font* button; the ruler follows Settings and moves with j and k; the reading set follows Settings › Reading. |

## Results — 2026-10-11

- `bunx vitest run`: 3,751 passed across 179 files.
- `bun run check`: zero errors, zero warnings.
