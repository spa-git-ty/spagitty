<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-041 — The pinned header cuts the detail card's shadow

**Status:** Fixed.
**Branch:** `bugfix/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow`
**Screens:** Graph (1A), with the commit detail open.
**Raised by:** the author, with a screenshot of the detail card's top-left
corner: "there's a part on top left, can you see it, look wrong?"

## Problem

Beside the detail card's top-left corner, the graph's column-header row showed
as a pale strip with hard edges. The pinned half of the column header
(`.header-frozen`) painted the pane's colour at all times. The detail card
floats in the pane and casts a soft shadow to its left; everything there was
tinted by it except that band, which sits above the shadow in the stacking
order. FEAT-083 made the pinned message cells opaque only while the graph is
scrolled under them, and did the same for the header's seam shadow, but left
the header's background opaque at rest.

## Scope

- The pinned half of the header is clear at rest and takes the pane's colour
  only while the graph is scrolled sideways, as the rows do.

## Acceptance criteria

- With the detail card open and the graph not scrolled sideways, the strip
  beside the card's corner is the same tint as the pane around it.
- Scrolled sideways, the graph still passes under the pinned header.
