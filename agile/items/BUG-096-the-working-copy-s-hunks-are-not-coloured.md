<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-096 — The working copy's hunks are not coloured

**Status:** Fixed — on `feat/FEAT-153-blame-on-click`; unmerged. Covered by a component test; not looked at in the running application (the author is checking it).
**Branch:** `feat/FEAT-153-blame-on-click`.
**Screens:** Working copy (1C).
**Raised by:** the author, 2026-10-20: "ther's a bug , in working copy the code is not colored".

## Problem

The Working copy's hunk pane drew each line as plain text with its changed words marked. Diff, Review and Files colour code by its language; the one screen where a person reads their own change before committing it did not.

## Fix

`HunkPane.svelte` reads each hunk with `tokenizeDiff`, the old and new sides apart as Review does, and cuts the colour at the changed words with `paint`, so a line carries both.

## Acceptance criteria

- Code in the Working copy's hunks is coloured as it is in Diff.
- Changed words are still marked, over the colour.
