<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-047 — Whole file scrolls back to the top

**Status:** Fixed.
**Branch:** `bugfix/BUG-047-whole-file-scrolls-back-to-the-top`
**Screens:** Review (1R), the review room.
**Raised by:** the author: "when i choose whole file and scroll it scroll back
up".

## Problem

The room opens a file at its top from an effect that calls
`VirtualRows.scrollToIndex(0, 'start')`. `scrollToIndex` read the list's row
offsets, which are derived from every row's measured height, while running. So
the effect depended on every measurement: each row drawn for the first time
while scrolling ran it again and sent the reader back to the top. Whole file
has the most rows to measure; Changes often had none left to measure once open.

The room's jump to a thread from the Conversation card called it the same way.

## Scope

- `scrollToIndex` reads its state untracked, so a caller's effect depends only
  on what the caller reads.
- No changelog entry: Review is unreleased.

## Acceptance criteria

- In Whole file, scrolling down a long file stays where it is put.
- Opening a file still starts at its top; a thread in the Conversation card
  still jumps to its line.
