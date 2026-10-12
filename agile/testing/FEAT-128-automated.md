<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-128 — Automated test record

**Item:** [FEAT-128](../items/FEAT-128-a-conflicts-columns-scroll-together-or-each-alone.md)

| Suite | Cases |
| --- | --- |
| `src/lib/resolver/scroll-link.test.ts` | `follow`: a column is put at the same share of its own length as the one scrolled; all are at their tops together and at their ends together, a little past the end included; a column with nothing to scroll stays at its top; a column is not moved on account of one that cannot scroll. `linker`: the other columns are put where the scrolled one is; a follower's own scroll event, a part of a pixel off where it was put, does not move the leader back; a person scrolling another column straight away leads from it, and the echo of that moves nothing; a column that did not have to be moved is not waited on. |
| `src/routes/merge/resolve.test.ts` | Mounted, in Merger, with the three columns told their sizes: **Scroll together** is on to start with, and a scroll on one side moves the result to the same share and leaves a side with nothing to scroll. Switched off, a scroll moves nothing else, and the choice is in `localStorage`. The resolver opened again is as it was left. |

## Not covered

- The card's height and the columns scrolling. The test environment does not lay a page out. They were measured on a page with the card's rules in headless Chrome 155, as the item records.
- A column's end not carrying on into the list (`overscroll-behavior`).

## Results — 2026-10-12

- `bunx vitest run`: 190 files, 4136 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run build`: succeeded.
