<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-051 — Automated test record

**Item:** [`agile/items/BUG-051-what-the-author-saw-on-first-run.md`](../items/BUG-051-what-the-author-saw-on-first-run.md)

| Suite | Cases |
| --- | --- |
| `src/lib/metrics.test.ts` | Widening spreads a squeezed history, never past the pitch; narrowing moves no lane that was apart; the cap and the compression thresholds at 16. |
| All suites | `bun run check` 0 errors; `bun run test` 3399 passed. |
