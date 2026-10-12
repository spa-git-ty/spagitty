<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-129 — Review shows a file before and after, side by side

**Status:** Done — on `release/2.2.2`, with BUG-079, BUG-080 and FEAT-128; unmerged. Covered by unit and mounted tests; not looked at in the running app.
**Screens:** 1R.
**Raised by:** the author, 2026-10-12, on 2.2.1: "on review, I see diff as parts or the whole file with diffs, can I have another view to see the whole file before and the whole file after".

## Problem

The review room reads a file two ways. **Changes** is its changed parts, with the unchanged lines between them folded. **Whole file** is every line, with the removed lines in place among the added ones. Both are one column: what was and what is are read interleaved, never apart.

## Change

- A third way, **Before and after**, beside **Changes** and **Whole file** in the pill.
- The file is set out twice: as it was on the left, under *Before*, and as it is on the right, under *After*. Each side is the whole file, in order, numbered by its own line numbers.
- An unchanged line is on both sides. A removed line is red on the left; an added line is green on the right. Where lines were removed and others added in the same place they are set beside each other, first with first, and the changed words are marked as they are in the other two views.
- Where one file has lines the other has not, the other side is left empty beside them.

## Decisions

- **One column, not two that scroll.** The two sides are two halves of each row, so they move as one and a line is always level with its counterpart. Two panes that scroll apart were considered, with a choice to link them as the resolver now has (FEAT-128). In a diff the reason to have the two sides at all is to compare a line with what replaced it, and two panes of different lengths cannot keep that level. It also keeps the room drawing only the rows in view, however long the file is.
- **It needs the files from disk**, as **Whole file** does, and is disabled with the same words when the pull request could not be fetched and only the host's patch is there.
- **Comments work in it.** A thread, a pending comment and the box one is written in are under the row their line is in, across both sides. A line is commented on from the side it is read on: the right, or the left for a line that was removed. An unchanged line has its `+` on the right only, since a comment on it is a comment on the new file.
- **The ruler and `j`/`k` go a row at a time**, on the line as it is now, or as it was where it is gone.
- **What a merge wrote while fixing a conflict is still a card of its own**, as it is in **Whole file** (FEAT-092).
- **The card's heading is *Before* and *After***, in place of the `@@` line. With the whole file shown, that line said only how long the file was.

## Acceptance criteria

- **Before and after** is offered beside the other two, and is disabled when the files cannot be read whole.
- Read down either side, the line numbers are that file's, in order, with none missing.
- A changed line is one row: the old line on the left, the new one on the right.
- Threads and pending comments are shown, and a new comment can be started.
- **One** and **All** work as in the other views.

## Not done

- The width is two equal halves. There is no divider to drag between them.
- Lines are paired by their order in a change, first removed with first added. A change that reorders lines pairs them as they come, not by which line became which.
- Nothing remembers the view between pull requests: a room opens on **Changes**, as before.

## What was done

`rows.ts`: `Scope` has a third value, `sides`, cut into blocks as `whole` is. `sideBySide(rows)` turns the flat list of rows into one where each line row is a `pair` row with a `before` and an `after`, keeping what is under a line under its pair, and closing each card on its last row whatever kind that now is. `holds` and `lineOfRow` find a line in either kind of row. `RoomDiff.svelte` draws a line from one snippet, used once for a line and twice for a pair. `RoomPill.svelte` has the third choice.
