<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-070 — The plan's file list scrolls, in a card the height of its neighbours

**Status:** Done — on `feature/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top`, with FEAT-124 to FEAT-127; unmerged. Seen in the built frontend against fixtures, drawn by headless Chrome; not yet in the running app.
**Screens:** 1S.
**Raised by:** the author, 2026-10-12, on 2.2.0, with two pictures. The pictures did not reach the session that built this; the request was read from its words and from the code.

## Problem

Under Merger's plan there are three cards in a row: *How it is merged*, *History after* and *What changes*. The row was as tall as its tallest card, and *What changes* lists every file either branch touched. A merge of forty files made that card forty rows tall, and the two beside it, which have a fixed amount to say, were stretched to the same height with empty space under them.

## Change

- The row is as tall as the taller of *How it is merged* and *History after*.
- *What changes* is that height, and its list of files scrolls inside it.

## Decision

`.changes` in `src/lib/merger/MergerPlan.svelte` has `contain: size`, so its content is left out of the row's height and the row stretches it to the height the other two set. The list inside is the flexible part and scrolls. When the window is narrow enough that the card wraps onto a line of its own there are no neighbours to take a height from, so it has a least height of 240 pixels.

## Acceptance criteria

- With many files, the first two cards are no taller than their own content needs.
- Every file can be reached by scrolling the list.
- With few files nothing looks different, except that a short list no longer sets the height either.

## What was done

Two rules in `MergerPlan.svelte`. Measured against a fixture of 34 files at 1440 by 900: the three cards are 365 pixels tall, and the list shows 307 pixels of 1364. At 900 wide the card wraps and is 240 pixels tall.
