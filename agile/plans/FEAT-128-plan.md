<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-128 — Plan

**Item:** [FEAT-128](../items/FEAT-128-a-conflicts-columns-scroll-together-or-each-alone.md)

**The height.** The card must know how tall the list is without script measuring it. The list becomes a size container, and the card's greatest height is `100cqh`: the list's height inside its padding, so the room the pill floats over is already left out. The card is a flex column whose children keep their heights, all but the grid of columns, which shrinks. The grid has one row, `minmax(0, 1fr)`, and its three columns stretch to it and scroll. Nothing is a percentage of a height that might not be definite.

Cards stop being shrinkable items of the list. A card that could shrink would be squeezed by the list in place of the list scrolling.

**The linking.** `scroll-link.ts`, apart from the component so it can be tested with no layout:

- `follow(source, target)` gives where `target` belongs: the same share of its own scrollable distance as `source` is at.
- `linker()` keeps a set in step. Moving a follower raises a scroll event on it, which would move the leader back by a rounding error; so whichever column the person scrolls leads, and for 120 milliseconds the others only follow.

The card registers its three columns with an action and listens for `scroll` on each. The resolver passes `linked` down and keeps it in `localStorage`, as the review room keeps whether its conversation is put away.

## What decides it is right

- On a page with these rules, a conflict longer than the list is no taller than it and its columns have more to scroll than they show.
- `follow` has the columns at their tops together and at their ends together.
- Mounted, a scroll on one column moves the others while **Scroll together** is on and does not while it is off.
