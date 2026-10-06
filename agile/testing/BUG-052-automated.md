<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-052 — Automated test record

**Item:** [`agile/items/BUG-052-squeezed-lanes-silent-waits-and-striped-notices.md`](../items/BUG-052-squeezed-lanes-silent-waits-and-striped-notices.md)

| Suite | Cases |
| --- | --- |
| `src/lib/metrics.test.ts`, `src/lib/graph/density.test.ts` | Sharing the span above the new floor; distinct x for every lane the floor allows; widening at the floor's pitch; compact compresses to its own floor. |
| `src/lib/ui/flat.test.ts` | The loader and toast read only defined tokens and name no colour of their own. |
| All suites | `bun run check` 0 errors 0 warnings; `bun run test` 3403 passed. |
