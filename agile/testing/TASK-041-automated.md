<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-041 — Automated tests

**Item:** [`agile/items/TASK-041-the-frame-stops-competing-with-the-work.md`](../items/TASK-041-the-frame-stops-competing-with-the-work.md)

## What was tested

20 new assertions in `src/lib/graph/density.test.ts`, 7 in
`src/lib/panels.test.ts`, 6 in `src/lib/nav.test.ts`. The suite went from 2,760
to 2,797.

**The largest piece of evidence in this task is what did *not* change.** Every
geometry function in `metrics.ts` gained a `density` parameter defaulting to the
values that were already there, and all 349 existing geometry and graph
assertions pass untouched. That is the proof that the comfortable density is
the same picture it was, and it is a stronger one than any new test here.

### The density

| Test | Asserts |
| --- | --- |
| starts comfortable | What every release before this drew. |
| persists and restores | Across a simulated relaunch, not just within a session. |
| ignores a stored value that is not a density | |
| survives storage being unreadable | |
| **stores a name rather than a geometry** | `{pitch: 16}` in storage would freeze one release's idea of compact into every installation that chose it. |
| says whether a node is big enough to be a face | The painter asks the preference, not a radius. |
| **gives an ordinary three-lane history most of its column back** | 149px against 72px: **77 pixels**, asserted rather than described. A "compact" worth twenty pixels would be a control nobody can see working. |
| still holds five lanes in less room than comfortable holds three | |
| is narrower at every depth | 1, 3, 5, 8, 12 and 20 lanes. |
| scales with zoom like every other metric | |
| *density* keeps a node inside its own pitch | The premise of the whole geometry — a lane closer than a node is wide draws lines through faces — checked for the new pair as well as the old. |
| leaves compact room to compress | Its resting pitch is above `LANE_PITCH_MIN`, or a deep history in compact mode would have nothing left to give. |
| floors the column at its own minimum | And shares the cap, which is a cap on width. |
| rests at its own pitch / node and shrinks from there | |
| *density* keeps every node inside the column it was given | **BUG-003's invariant**, re-checked at both densities across five column widths and five depths. |
| starts both densities at the same first lane | The column's left edge does not move, so rows and canvas agree without knowing the density. |

### Adaptive widths

| Test | Asserts |
| --- | --- |
| is the design width in a wide window | |
| gives the work more room in a narrow one | |
| never starts a panel at the width it stops being useful at | The floor is not `min`. |
| is monotonic in the window width | 900 → 1920, never going backwards. |
| **never overrides a width somebody chose** | A dragged width is a decision, including after the window moves. |
| leaves a bottom drawer alone | A drawer's height has nothing to do with window width. |
| scales every side panel, not only the two named ones | |

The existing suite is now pinned to a 1600px window in `beforeEach`, because
happy-dom reports 1024 and every assertion naming `RAIL_W` is about the design
value rather than a narrow window's version of it.

### The rail

`covers every item exactly once, in the same order` is the important one:
grouping must not move anything. Plus the group boundaries, the headings, that
the Farm's group has exactly one row and no heading, that each group is one
contiguous run, and that the two screens which are not about the open repository
are together.

## What the tests caught that the reasoning did not

**The node started following the dragged column again.** Threading the span into
`laneNodeRadius` looked obviously right and silently reversed FEAT-039, which
had decided the opposite on purpose. Two existing assertions — `draws the same
size node at every column width it can be dragged to` and `shrinks by depth
alone, so the drag cannot change it` — failed immediately. The fix was
`laneSpanOf(density)`: a reference span that belongs to the density rather than
to the drag.

That is the second time in this run of work that an existing test written years
earlier caught a regression in reasoning rather than in code. It is worth
recording as evidence that the geometry's assertions are earning their keep.

## What is not covered, and why

- **Whether compact looks right.** Every number in it is reasoned and asserted;
  none of it has been seen on a screen. `SWEEP-006` to `SWEEP-009`.
- **Whether 40px is enough for the toolbar.** Arithmetic says 38 plus air.
  `SWEEP-001` is a look at it.
- **Whether the rail reads as four groups.** `SWEEP-003`.
- **The reduced-motion fix.** `gentleFly` is a two-line function over
  `matchMedia`; that Svelte then does not animate is `SWEEP-010`.

## Coverage

`src/lib/graph/density.svelte.ts` and `src/lib/motion.ts` are first-party and
covered by their own assertions — every branch of the store, and both sides of
the motion check. `metrics.ts` gained parameters rather than statements; its
existing tests cover the defaults and `density.test.ts` covers the other value.
