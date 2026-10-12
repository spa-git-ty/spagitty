<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-129 — Automated test record

**Item:** [FEAT-129](../items/FEAT-129-review-shows-a-file-before-and-after-side-by-side.md)

| Suite | Cases |
| --- | --- |
| `src/lib/review/rows.test.ts` | `blocksOf` cuts `sides` as it cuts `whole`. `sideBySide`: an unchanged line is on both sides and a changed one beside what replaced it, so thirty lines are twenty-nine rows and only the last closes the card; each side reads 1 to n in order; a side is empty beside lines the other file does not have, with two removed for one added and two added for none; a thread stays under the row its line is in and inside its card, and closes the card when it is under the last line; a line is found by its key on either side, a pair is read by the line as it is now, and `j` and `k` have one stop a row; rows that are not lines are left as they are. |
| `src/routes/review/room.test.ts` | Mounted: **Before and after** is pressed and shows *Before* and *After* with no fold and no `@@` line; the file's thirty lines are twenty-nine pairs; the edited line is one row, removed on the left and added on the right, each numbered 15; an unchanged line is on both sides with its `+` on the right only, and the removed line has its own; back to **Changes** the pairs are gone and the folds are back. Read from the host's patch, **Before and after** is disabled with **Whole file**. |

The room's other mounted tests pass unchanged: the ruler, comments, threads, jumps and conflict-fix cards in **Changes** and **Whole file** are drawn by the same snippet the pairs use.

## Not covered

- Writing a comment from the side-by-side view, and jumping to a thread in it. The pairs hold the same line rows the other views use, and the lookups they share are tested in `rows.test.ts`; neither was driven end to end.
- How the two halves look. Not drawn in a browser.

## Results — 2026-10-12

- `bunx vitest run`: 190 files, 4134 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run coverage`: passed its thresholds (lines 85.77%).
- `bun run build`: succeeded.
