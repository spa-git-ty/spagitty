<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-128 — A conflict's columns scroll in place, together or each alone

**Status:** Done — on `release/2.2.2`, with BUG-079, BUG-080 and FEAT-129; unmerged. The layout was measured in headless Chrome on a page that copies the card's rules, and the linking is covered by tests; not looked at in the running app.
**Screens:** 1D, 1S.
**Raised by:** the author, 2026-10-12, on 2.2.1, with a picture of Conflicts: "the window should not all be scrollable! only the files or diff buffer should be scrollable (and choose whether magnet them together or scroll each alone)".

## Problem

A conflict is a card: its header, three columns (one side, the result, the other side), and under them the ways to resolve it. The card was as tall as its longest column, and the list of cards scrolled.

In the picture a changelog conflicts: one side is forty wrapped lines. The card is several windows tall. To read down a side the whole card is scrolled, its header and the column headings go off the top, and the choices at its foot are far below, out of sight until the end is reached. The three columns can only move as one, by the same distance, though they are three texts of three lengths.

## Change

- A card is never taller than the room the list has for it. A long conflict scrolls inside its three columns, and its header and its choices stay where they are.
- **Scroll together**, in the pill beside **Base**, is on to start with: scrolling one column moves the other two. Off, each column scrolls alone.
- The choice is remembered on this machine.
- A short conflict looks as it did, except that its three columns are now one height.

## Decisions

- **Together means by the same share, not the same distance.** One side may be forty lines and the other four. Moved by the same number of pixels, the short one would be at its end while the long one is a third of the way down. Moved by the same share of what each has to scroll, all three are at their tops together and at their ends together. A column with nothing to scroll stays where it is.
- **The list still scrolls from one conflict to the next.** The author asked that the window not scroll. With one conflict in a file, as in the picture, it no longer does: the card fits and only the columns move. A file with several conflicts is still a list of cards, and the list moves between them; none of them is taller than the list. Showing one conflict at a time was considered and not done: the list is how two short conflicts are seen at once, and the pill already steps through them.
- **A column's end does not carry on into the list.** Reaching the bottom of a column with the wheel does not start the list moving under the pointer.
- **The Base strip scrolls too.** Shown for a long conflict, it would have taken the room the columns need, so it is held to under a third of the list's height.
- **The toggle is in the pill**, with the other things that are about how the conflict is shown.

## Acceptance criteria

- A conflict longer than the list is as tall as the list, less a margin, and its columns scroll.
- Its header, the column headings and its choices do not move when a column is scrolled.
- With **Scroll together** on, scrolling one column puts the others at the same share of their own lengths. With it off, they stay.
- The choice survives leaving the resolver and coming back.
- Merger and Conflicts behave the same: it is one resolver.

## Not done

- Lines are not held level across the columns. The three wrap differently and the result has lines neither side has, so there is no one alignment to keep; together is by share.
- The files list on the left is unchanged. It already scrolled on its own.
- A card has a least greatest-height of 240 pixels: its header, its choices and the least its columns can be. In a window so short that the list has less than that, the list scrolls the card, as it did before.

## What was done

`ConflictCard.svelte`: the card is a column with a greatest height of the list's height (`100cqh`, the list being a size container) less 12 pixels; everything in it keeps its height except the three columns, whose row takes what is left and whose columns scroll. Each column reports its scrolling to `scroll-link.ts`, which moves the others while the card is linked. `Resolver.svelte` holds the choice, in `localStorage` under `spagitty.resolver.scrollTogether`, and makes the list a size container. `ResolverPill.svelte` has the toggle, and a new `link` icon.

Measured in headless Chrome on a page with the card's rules, in a list 560 pixels tall with 100 of padding for the pill: a conflict of eighty lines is 448 pixels tall and its first column scrolls 1608 pixels of text in 348; a conflict of six lines beside it is as tall as its text.
