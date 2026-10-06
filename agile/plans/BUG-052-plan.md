<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-052 — Plan

**Item:** [`agile/items/BUG-052-squeezed-lanes-silent-waits-and-striped-notices.md`](../items/BUG-052-squeezed-lanes-silent-waits-and-striped-notices.md)

`metrics.ts`: `LANE_PITCH_MIN` 22, `COMPACT_PITCH_MIN` 14, `lanePitch` picks the floor by density. `src/lib/ui/Loader.svelte` (SVG strands with `offset-path` nodes; inline bars), swapped in for each loading text by a script over `src/**/*.svelte`; `Btn` `busy`. `NoticeToast.svelte` on `.ornament` with a filled mark.
