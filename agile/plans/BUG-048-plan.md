<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-048 — Plan

**Item:** [`agile/items/BUG-048-a-wider-graph-column-draws-no-more-lanes.md`](../items/BUG-048-a-wider-graph-column-draws-no-more-lanes.md)

## Approach

Every object on a lane — track, crossings, node, stash ghost, hover target — is
placed by `laneX`, so the fix is there alone. The pitch still comes from the
resting span: recomputing it from the dragged span is the squeeze FEAT-081
removed, and it would move lanes that already fit. Only the cap changes, from
`min(resting, span)` to `span`. That is continuous and monotonic in the drag, so
FEAT-081's no-threshold and one-pixel-per-pixel properties carry over.

`span` becomes optional and defaults to the density's resting span. The painter
and `LaneCanvas` stop defaulting it to `LANE_SPAN`, which at the compact density
would now spread the deepest lanes 110 px further than an undragged column.

## Files

| File | Change |
| --- | --- |
| `src/lib/metrics.ts` | `laneX` caps at the span only; `span` defaults to the resting span. |
| `src/lib/graph/lanes.ts` | `span` passes through undefined rather than `LANE_SPAN`. |
| `src/lib/graph/LaneCanvas.svelte` | The same. |
| `src/lib/metrics.test.ts` | Widening tests; the undragged-column test compares against no span. |
| `CHANGELOG.md` | Unreleased › Fixed. |

## Risks and rollback

- An undragged column whose width rounded up by half a pixel now puts its
  stacked lanes half a pixel further right. They are still inside the column;
  the test states the bound as 0.5 px rather than `toBeCloseTo`, which is
  strictly under it.
- Rollback is a revert.
