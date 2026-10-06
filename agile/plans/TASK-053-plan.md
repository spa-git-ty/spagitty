<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-053 — Plan

**Item:** [`agile/items/TASK-053-the-review-room-after-first-use.md`](../items/TASK-053-the-review-room-after-first-use.md)

## Approach

- **Who wrote it.** `RoomFiles` counts what `room.isAuthors` and `room.hasFix`
  let through for the chips, and draws `author` beside `conflict fix` while
  the conflict fixes are known (the head was fetched).
- **One number.** The line grid loses a gutter column; the one left shows
  `new ?? old`, with `.old` on a removed line. Threads, drafts and the
  composer are indented by one gutter less.
- **Whole file.** `changedRanges` returns its ranges with their `fixed` flag.
  In `whole`, `blocksOf` lays the file end to end as `hunk` blocks, cut only
  around the fixed ranges, so a file with no fix is one block. It keeps a
  `@@` header, which gives the card its top.
- **The pill.** `RoomPill` takes `onfinish` from `ReviewRoom`, which owns the
  Finish card. `Viewed` is `aria-pressed` by `room.isViewed`; not viewed it
  calls `viewedNext`, viewed it calls `setViewed(path, false)`. With every file
  viewed it is `Finish review`. Two icons, `circle` and `circle-check`.

The author chose one number column over two (asked 2026-10-05), and said the
Author problem was the filter chip.

## Files

| File | Change |
| --- | --- |
| `src/lib/review/RoomFiles.svelte` | Counts on the chips; `author` on files. |
| `src/lib/review/RoomDiff.svelte` | One number column. |
| `src/lib/review/rows.ts` | Whole file as one part, conflict fixes apart. |
| `src/lib/review/RoomPill.svelte` | `Viewed` with a ring; `Finish review` once all are viewed. |
| `src/lib/review/ReviewRoom.svelte` | Opens the Finish card for the pill. |
| `src/lib/ui/icons.ts` | `circle`, `circle-check`. |
| `src/lib/review/rows.test.ts`, `src/lib/review/fixes.test.ts`, `src/routes/review/room.test.ts` | Tests below. |

## Risks and rollback

- Whole file's one card has no fold and no break, so a long file is one long
  card; the list is virtual, so it costs what the cards did. Rollback is a
  revert.
- The Diff screen keeps two number columns; only the room changed.
