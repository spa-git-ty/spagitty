<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-069 — Added lines sometimes look red

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** 1R, 1B.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## What happens

Some added lines read as red. Additions must read as green and removals as red, every time, in every theme.

## Suspected cause

Not reproduced yet. In `src/lib/review/RoomDiff.svelte`, three things can put red on an added line:

1. **The focus ruler and the comment range replace the green row.** `.line.focused` (`--ruler`) and `.line.covered` (`--accent-soft`) are declared after `.line.added` and override its background. The accent is the tomato (`#b8321f`), so a commented range of added lines is tinted red.
2. **Markup tag names are red.** `.tok-tag` is `--lane-1` (`#c23b22`) in `src/app.css`. On the calm colours, the added tint is only 8% green, so an added line of HTML or Svelte reads as red text.
3. **The comment button.** Each line's `+` is accent-coloured, so it is red too.

## Acceptance criteria

- An added line's colour is always green and a removed line's always red. The ruler, a comment range and syntax colours sit on top of that colour and never replace it.
- No syntax colour is the same red as a removal. Tag names move to another lane.
- The calm colours are strong enough that an addition is read as green at a glance, checked in light and dark.
- The same rules hold in the Diff screen and the Pull requests diff.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Reproduce first:** ask the author for the file and theme, and check each suspect above against it.
- **Where:** `src/lib/review/RoomDiff.svelte` (the `.line` rules), `src/app.css` (`--diff-*`, `--ruler`, `.tok-*`), `src/lib/requests/PRDiffPane.svelte`, the Diff screen.
- **Branch:** `bugfix/BUG-069-added-lines-look-red`, with plan, testing documents and a changelog entry when the work starts.
