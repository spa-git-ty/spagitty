<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-080 — Plan

**Item:** [BUG-080](../items/BUG-080-the-review-pill-runs-out-of-a-narrow-diff.md)

Fix both causes, because either alone leaves the picture possible.

**The pill.** Place it with `left` and `right` both set and `margin-inline: auto`, and let its width be `fit-content`. The box it is laid out in is then the column less its margins, not half of it. Group its parts so a row breaks between groups and not inside one, and let it wrap. Its corners become a fixed radius, half the height of one row, since a fully round end on two rows would cut into what is in them.

**The room.** Give the diff a least width and no shrink, and let the two panels shrink to the least widths their dividers allow. Flex does the arithmetic: nothing is measured in script, and nothing is written back to the saved widths, so a panel dragged wide in a wide window is still that wide when the window is wide again.

Not done by lowering the panels' greatest widths: 720 for the conversation is right in a wide window, and the fault is what happens in a narrow one.

## What decides it is right

- In a room 1190 wide with the conversation at 690, the diff is 560 and every part of the pill is inside the pill.
- In a room wide enough for all three, the panels are the widths they were dragged to.
