<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-129 — Plan

**Item:** [FEAT-129](../items/FEAT-129-review-shows-a-file-before-and-after-side-by-side.md)

The room already has everything the view needs: a file read whole is every line of the new version with the removed lines in place, each with its old and new number, its syntax and its changed words. Nothing new is asked of the core.

So the view is a second arrangement of the same rows, made after `rowsOf` and before the list draws them:

1. `blocksOf` treats `sides` as it treats `whole`: one part per file, a conflict fix apart.
2. `rowsOf` is unchanged. It gives line rows with threads, pending comments and the composer after the lines they belong to.
3. `sideBySide` walks those rows. An unchanged line becomes a pair of itself. A run of removed lines followed by a run of added lines becomes pairs, first with first, with `null` on the shorter side. Rows under a line are kept after the pair that line is in. Other rows pass through.
4. The list draws a pair as a grid of two of the same line.

Because a pair holds the line rows themselves, focus, the comment range and jumping to a thread keep working by a line's key; they ask `holds(row, key)` where they compared keys.

Not done as a second component with its own scrolling: the virtual list, the reading ruler, comments and conflict-fix cards would each have had to be built twice.

## What decides it is right

- `sideBySide` of a file with one edited line has one row fewer than the file has lines, and each side reads 1 to n.
- A side with fewer lines in a change is empty beside the rest.
- A thread stays under its line and inside its card.
- Mounted, the room shows the old line beside the new one and returns to **Changes** with its folds.
