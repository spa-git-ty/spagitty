<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-048 — Automated test record

**Item:** [`agile/items/BUG-048-a-wider-graph-column-draws-no-more-lanes.md`](../items/BUG-048-a-wider-graph-column-draws-no-more-lanes.md)

## What was tested

`src/lib/metrics.test.ts`, *widening the graph column*, at `git/git`'s mean
depth of 187 lanes:

- in a 650 px column lane 30 has its own x, 14 px from lane 29, where at rest it
  was stacked on the span's end; the deepest lane folds onto the new edge;
- a lane already apart at rest does not move at 400, 650 or 1 200 px, for
  histories of 3 to 187 lanes;
- from 1 200 px down to 332 px, at both densities, no lane moves more than the
  pointer did and every full-size node stays inside the column;
- a compact column given no span rests where an undragged one does, not
  110 px further out.

*Changes nothing for a column nobody has dragged* now compares against no span
rather than `LANE_SPAN`, and states its rounding bound as 0.5 px.

## Test command and output

On Windows 11: `bunx vitest run` — 3 249 passed, 1 failed. The failure is
`tools/record.test.ts`: BUG-043 is Fixed and lacks its plan and testing
documents. It fails the same way before this change and is not part of it.

`bun run check` — 0 errors, 0 warnings.

## What is not covered automatically

How the lanes look in the release build. See the sweep.
