<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-080 — The review pill runs out of a narrow diff, over the conversation

**Status:** Fixed — on `release/2.2.2`, with BUG-079, FEAT-128 and FEAT-129; unmerged. The layout was measured in headless Chrome on a page that copies the room's rules; not looked at in the running app.
**Screens:** 1R, 1D, 1S.
**Raised by:** the author, 2026-10-12, on 2.2.1, with a picture of the review room: "this bar is wrong".

## What happens

In the picture the window is 1266 wide and the conversation has been dragged to about 690. The diff column between the files and the conversation is left about 190 wide. The pill at its foot is drawn 168 wide: *1 of 3* sits under the two arrows, and **Changes**, **Whole file**, **One**, **All** and **Viewed** run out of the pill and across the box the overall comment is written in.

## Cause

Two things.

- **The pill had half its column to be laid out in.** It was centred with `left: 50%` and a transform. An absolutely placed box that is given only `left` is as wide as its content up to what is left of the column to its right, which is half. Anything wider was squeezed, the file's name first because it was the one part allowed to shrink, and then the pill was clipped to the column while its buttons were not.
- **Nothing kept the diff column wide enough.** The files and the conversation had fixed widths, each draggable to 560 and 720, and the diff took what was left. In a window of ordinary width the two could leave it a strip.

The resolver's pill was centred the same way and had the same fault in a narrow column.

## Acceptance criteria

- The pill is never wider than its column, and nothing in it is drawn outside it.
- The file's name and its place are not squeezed: the name is cut with an ellipsis at its own width.
- In a column too narrow for one row, the pill's groups go onto a second row.
- The diff column is not narrower than 560 pixels while the files and the conversation have width to give. They give way to their least widths, 180 and 240, and come back when there is room.
- The same holds for the resolver's pill.

## Fix

- `RoomPill.svelte` and `ResolverPill.svelte`: the pill is placed between the column's two edges and centred by its margins, with `width: fit-content`, so it has the whole column to be laid out in. It wraps. The name and place no longer shrink.
- `ReviewRoom.svelte`, `RoomFiles.svelte`, `RoomConversation.svelte`: the diff has a least width of 560 pixels and does not shrink; the two panels beside it do, to the least widths their dividers already allow. The width a panel was dragged to is kept, and is what it returns to.
- `RoomDiff.svelte` leaves a little more room under the last row, for a pill of two rows.

## Measured

On a page with the room's layout rules, in headless Chrome, with the files at 262 and the conversation at 690:

| Room | Before | After |
| --- | --- | --- |
| 1190 wide | diff 222; the pill's buttons outside it | files 180, diff 560, conversation 434; the pill inside it, two rows |
| 1600 wide | — | files 262, diff 632, conversation 690; the pill inside it, two rows |
| 700 wide | — | files 180, diff 560, conversation 240 |

## Not done

- The pill is one row only in a diff column of about 740 pixels or more, now that it has a third way to read a file (FEAT-129). Narrower than that it is two rows. Shorter words or icons in place of words would make it one; that is a change to what the pill says, and was not made here.
- The room does not scroll sideways below 1016 pixels of room, where the three least widths no longer fit. The window's own least width is what prevents that.
