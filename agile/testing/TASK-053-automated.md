<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-053 — Automated test record

**Item:** [`agile/items/TASK-053-the-review-room-after-first-use.md`](../items/TASK-053-the-review-room-after-first-use.md)

## What was tested

- `src/lib/review/rows.test.ts`, *shows an expanded fold, and the whole file as
  one part*: Whole file on a file with one edit is one block.
- `src/lib/review/fixes.test.ts`, *keeps a fix a card of its own in the whole
  file too*: the fix is cut out as in Changes; the rest lies either side.
- `src/routes/review/room.test.ts`:
  - *shows the whole file as one card, with nothing folded*: one card top, no
    `unchanged ·` header.
  - *numbers each line once*: one number per line; a removed line shows its
    old number, marked `old`.
  - *says on the pill whether the file is viewed, and offers Finish once all
    are*: the ring is pressed on a viewed file and pressing it takes the tick
    back; with both files viewed the pill shows Finish review, which opens the
    card.
  - *lists the files by who wrote them*: `author` on the author's file only;
    `Author 1`, `Conflict fixes 1`, `All 2`.
  - Two existing tests now press the pill's `Viewed`, and take a line's one
    number for a shift-click.

All seven new or changed checks failed before the change.

## Test command and output

On Windows 11: `bunx vitest run src/lib/review src/routes/review src/lib/ui` —
257 passed. `bun run check` — 0 errors, 0 warnings.

Seen in headless Chrome on a preview route with a six-file pull request (not
committed): one number column, `author` marks and counted chips; Whole file one
card with the conflict fix framed apart, staying at 900 px when scrolled; the
pill's hollow ring, its pressed ring on a viewed file, and Finish review at 6
of 6 opening the card.

## What is not covered automatically

The look against a real pull request in the release build. See the sweep.
