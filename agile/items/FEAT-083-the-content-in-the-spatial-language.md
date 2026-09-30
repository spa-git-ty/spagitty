<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-083 — The content in the spatial language

**Status:** Open — merged into `main` on 2026-09-30; the manual sweep is still owed.
**Branch:** `feature/FEAT-083-the-content-in-the-spatial-language`
**Screens:** every screen's header; the Graph (1A) above all — its column
header, rows, working-copy row and detail panel; branch and tag chips.
**Raised by:** the author, after FEAT-082: "change the graph area or content
area to match the spatial design; it looks like it's borrowed from another
place, not matching the design philosophy of spatial design."

## Problem

FEAT-082 made the shell spatial and left every screen as it was, as asked at
the time. Inside the new pane, the old screens are still a stack of flat bands:
a filled header bar with a rule, a filled column-header bar with a rule and a
line between every column, a shaded band down the graph, full-width stripes for
hover and selection, a working-copy row that is another band, and a detail
panel cut off by a hard line. Against glass ornaments it reads as a different
application pasted in.

## Change

- **The bands step back.** Inside the pane, `--chrome-veil` is transparent and
  `--graph-bg` is the pane's own colour, so every screen's header and the
  graph's column are one surface with the controls on it — without editing
  seventeen screens. `--graph-bg` is also the ring the lane canvas cuts round a
  node, which is why it becomes the pane colour rather than transparent.
- **Columns are separated by space.** No rules between the message, author and
  date cells; the column header draws its dividers only while hovered, in the
  secondary colour, over the faintest rule.
- **Rows highlight as capsules.** Hover and selection are an inset, rounded
  highlight drawn by a pseudo-element under the cells and the lanes. The row
  itself does not move, so nothing the lane canvas is laid out against changes.
  The pinned message cells paint the pane colour only while the graph is
  scrolled under them, and the seam's shadow shows only then.
- **The working copy is a card**, inset, in the ornaments' glass.
- **The detail panel is an inspector card** floating in the pane, with its own
  corner and the glass edge.
- **Chips are capsules.** Branches are full capsules; tags keep a distinct
  shape — square at the start, round at the end.

## Non-scope

- The lanes, nodes and the graph's geometry.
- Screens other than the Graph beyond what the two tokens change; each can get
  its own pass.

## Acceptance criteria

- With a repository open, the Graph screen shows no filled bar across its top,
  no rule between columns at rest, and no band down the graph.
- Hovering and selecting a row draws a rounded, inset highlight; the lanes and
  nodes do not move by a pixel.
- Scrolling the graph sideways under the pinned messages still hides it, with
  the seam's shadow.
- The detail panel is a card with its own corner inside the pane.
