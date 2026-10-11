<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-120 — Automated test record

**Item:** [FEAT-120](../items/FEAT-120-the-review-file-list-hides-and-has-two-sizes.md)

| Suite | Cases |
| --- | --- |
| `src/routes/review/room.test.ts` — *the file list (FEAT-120)* | The list is put away by its button and by f and returns from the edge tab with the diff still there; f typed into a box is ignored; Compact drops the facts for dots and keeps the tick, counts and selection; Detailed returns; the legend waits in Compact; both choices are saved and read again after a restart. |

## Results — 2026-10-11

- `bunx vitest run src/routes/review`: 61 passed. `bun run check`: zero errors, zero warnings.
