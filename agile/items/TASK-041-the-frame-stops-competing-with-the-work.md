<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-041 — The frame stops competing with the work

**Status:** Done.
**Screen:** chrome (the toolbar and the rail), the Graph screen's column, and
every screen that opens a side panel.
**Raised by:** a UI review: "the application frame competes with the work", and
"at 1280px, opening detail leaves roughly 489px for messages and their
metadata".

## Problem

Four separate measurements, one symptom: a window where the application takes
more room than what is in it.

**110 pixels of chrome before the screen's own header.** `metrics.ts` reserves a
30px title bar, a 30px tab strip and a **50px** toolbar, on a window whose
default height is 800. The toolbar was the part with slack in it: its controls
are an icon over a label, and the two together measure 15px + 16px at the
application's smallest type, plus 3px of padding at each end. Thirty-eight. The
other twelve pixels were not doing anything.

**Fourteen destinations, one divider, and identical treatment for all of them.**
That is a list, not a structure. Supervising a farm of agents, doing routine git
work, reaching for a tool once a fortnight, and going somewhere that is not
about this repository at all are four different activities, and the rail gave
the eye no way to tell them apart without reading every label.

**791 pixels spoken for before a word of a commit message.** On the 1280 window
the application opens at, the Graph screen reserves the rail at 186, the refs
gutter at 186, five lanes' worth of column at 149 and the detail panel at 270.
That leaves 489 for the subject line and its metadata, on a screen whose entire
job is reading subject lines. The handoff's numbers are a 1440-wide window's
numbers and were being applied to every window.

**Five lane columns reserved for a three-lane history.** `laneColumns` floors at
`LANE_COLUMNS_MIN`, which is five, so the ordinary repository — two or three
lanes deep — pays for two lanes it does not have. At the design's own metrics
that is 77 pixels.

**And one thing that was not a measurement.** `+layout.svelte` ran an
unconditional `in:fly` on every navigation, with a comment saying
`prefers-reduced-motion` turned it off in `app.css`. It did not and could not: a
Svelte transition is driven from JavaScript, writing a new transform every
frame, so there is no CSS transition for the media query to shorten. Somebody
who had asked their machine to stop moving things got the slide on every single
navigation, and the evidence that they did not was a rule that could not see it.

## Change

- **The toolbar is 40px**, not 50. Two pixels of air above what its controls
  measure, and it scales with zoom like every other metric here, so 150% gets a
  60px bar rather than a 40px bar with 24px controls in it. Going further means
  taking the labels off, which the narrow layout already does below 900px —
  the case where that trade is worth making.
- **The rail is four groups**: the Farm alone at the top, then `Repository`,
  `Tools` and `Spagitty`. The **order is unchanged**, deliberately: grouping the
  rows must not move them, so whatever a hand has learned about which position
  is which screen still holds. Expanded, each group after the first carries a
  quiet heading; collapsed, the same boundary is the divider it already was.
  `navRows()` computes the boundaries, so the rail renders a list and
  `nav.test.ts` asserts the shape without mounting anything.
- **Panel widths adapt to the window they first open in**, between 1100 and
  1440, and **never override a width somebody dragged** — `adapt()` runs before
  the stored values are read, so each stored one simply overwrites its default.
  The floor is two-thirds of the way from a panel's minimum to its design width
  rather than the minimum itself: starting at the minimum would open every panel
  at the edge of usefulness with nowhere left to go.
- **A compact graph density.** `metrics.ts` gains a `Density` — pitch, node
  size, and the lane-count floor — as one set rather than three settings,
  because they are not independent: the whole argument for the 26px pitch is
  that a lane closer than a node is wide draws lines through faces. `compact` is
  a 16px pitch, a 6px node and a floor of three columns, which gives an ordinary
  three-lane history **77 pixels** back. Every geometry function takes a density
  and defaults to `comfortable`, so nothing that existed before this change
  behaves differently.
- **`gentleFly`** in `src/lib/motion.ts` asks the reduced-motion question in
  JavaScript, where CSS cannot. The duplicate `prefers-reduced-motion` block in
  `app.css` — two copies with different durations, one 0.01ms and one 0.001ms —
  is now one.

## What compact costs, said plainly

The node stops being a portrait and becomes a mark. That is the trade being
offered rather than a bug: the picture is still drawn wherever there is room for
a face to be a face — the author column, the commit detail — which is FEAT-079's
own argument about where a picture belongs. `density.portraits` says which mode
is which, so the painter asks the preference rather than comparing a radius
against a magic number.

## Acceptance criteria

- The chrome above a screen's header is 100px rather than 110, at every zoom.
- The rail shows four groups in the original order, with headings expanded and
  dividers collapsed.
- A 1280 window opens with visibly narrower panels than a 1600 one, and a
  dragged width survives both.
- Compact graph gives a three-lane history at least 70px back, keeps every node
  inside its own column, and leaves room to compress a deep history further.
- A navigation with reduced motion enabled does not move.

## Non-scope

- **The tab strip and title bar heights.** Both are 30px and both are carrying
  their content. Merging the two rows is a bigger change than this and would
  need the window controls redesigned with it.
- **Per-screen polish.** The review's stage 3 asks for before/after captures of
  Graph, Working copy, Diff and Farm at identical size and data. This task
  changes the geometry those captures would be of; the captures themselves need
  a running window and are in the sweep.
- **The refs gutter.** Fixed at 186 and not resizable. It is the next candidate
  and it is a separate change, because it shares its geometry with the rows.

## Dependencies

FEAT-035, FEAT-039 and FEAT-046 built the lane geometry this extends; their
decisions — a cap on width rather than on lanes, a node sized by depth rather
than by the drag — are preserved and asserted at both densities. FEAT-040 put
the divider before All repositories that the `Spagitty` group's heading now
replaces. BUG-003 is the invariant the density tests re-check.
