<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-051 — Plan

**Item:** [`agile/items/BUG-051-what-the-author-saw-on-first-run.md`](../items/BUG-051-what-the-author-saw-on-first-run.md)

## Approach

`MergerPlan.svelte`: the head, stage, lands and lower rows are `flex: none` in the plan's scrolling column, which had let them shrink below their content. `plan.ts` `stats`: zeros when nothing comes in. `Resolver.svelte`: the folder under each file's name. `RefTabs.svelte`: the active screen's title only. `metrics.ts`: `LANE_COLUMNS_MAX` 16, and `laneX` compresses against the larger of the resting and the dragged span.
