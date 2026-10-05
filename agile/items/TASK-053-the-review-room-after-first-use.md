<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-053 — The review room after first use

**Status:** Done.
**Branch:** `task/TASK-053-the-review-room-after-first-use`
**Screens:** Review (1R), the review room.
**Raised by:** the author, 2026-10-05, after reviewing pull requests in the
room for the first time (FEAT-091, FEAT-092).

## Problem

What the author found, in their words:

- "author not appear when i click" — the Author filter. On a pull request
  with no conflict fix it lets every file through, so choosing it changed
  nothing, and no file said it was the author's: the legend's grey dot was on
  none of them.
- "numbers appears twice on lines" — every line carried its old and its new
  number side by side, which on an edited file reads `4 4`, `5 5`.
- "show diff in whole file option area not separated" — Whole file cut the
  file into cards: each change, and each unchanged run between them under an
  `unchanged · 9–152` header.
- "viewd,Next looks not good" — the pill's solid green `✓ Viewed, next`.
- "when i finish viewd next is still viewd next not turned to finsh".

## Scope

- Each file the author changed says `author`, with the legend's grey dot, as a
  conflict fix says `conflict fix`. The filter chips say how many files each
  lets through: `All 6`, `Author 5`, `Conflict fixes 2`.
- One number per line: the new file's, or the old file's on a removed line,
  quieter.
- Whole file is one card with the changes in place. A conflict fix stays a
  card of its own, framed in sky, so it is never read as the author's work
  (FEAT-092).
- The pill's button is `Viewed` with a ring: hollow while the file is still to
  read — pressing it ticks the file and opens the next — and ticked, in green,
  once read, when pressing it takes the tick back.
- Once every file is viewed the button is `Finish review`, which opens the
  Finish review card.
- No changelog entry: Review is unreleased, and its *Added* entries still
  describe it.

## Acceptance criteria

- With the conflict fixes read, every file the author changed shows `author`;
  the chips show their counts.
- No line shows two numbers.
- Whole file on a pull request with no conflict fix is one card per file.
- The pill shows a hollow ring on a file not viewed and a ticked green ring on
  one viewed.
- With every file viewed the pill offers Finish review, and it opens the card.
