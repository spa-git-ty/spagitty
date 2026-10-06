<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-048 — A wider graph column draws no more lanes

**Status:** Fixed.
**Branch:** `bugfix/BUG-048-a-wider-graph-column-draws-no-more-lanes`
**Screens:** Graph (1A).
**Raised by:** the author, on `git/git`: "why in main graph the graph has limit
it cant extend more to right although i dragged more space for it".

## Problem

`laneX` shares a deep history's lanes out across the density's resting span —
286 px at the comfortable density — down to the 14 px pitch floor, and stacks
every lane past the span's end on it. It then capped that offset at the dragged
span as well. The cap only ever bit in one direction: a column dragged narrower
folded lanes onto its edge (FEAT-081), but a column dragged wider than it rests
was still capped at the resting span, so the lanes stopped at the same x and the
rest of the column stayed empty.

## Scope

- The dragged span is the only cap. Lanes keep the pitch the resting span gave
  them, so a lane already apart never moves; widening releases the stacked ones
  one by one, as narrowing folds them.
- Left out, the span is the density's resting span rather than `LANE_SPAN`, so
  an undragged compact column still rests where it did.
- A changelog entry under Unreleased › Fixed: the graph is released.

## Acceptance criteria

- On `git/git`, dragging the Graph column wider draws lanes further right, up to
  the column's edge.
- Lanes already apart do not move while the column is widened or narrowed past
  them.
- Narrowing still folds lanes onto the edge, and an undragged column looks as it
  did, at both densities.
