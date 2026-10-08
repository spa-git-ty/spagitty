<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-060 — Automated test record

**Item:** [BUG-060](../items/BUG-060-merger-forecast-lost-on-first-open.md)

| Suite | Cases |
| --- | --- |
| `src/lib/merger/store.test.ts` | Three refreshes before the forecast answers still let it land, with one request; a moved tip asks again and only the newer answer is kept; a settled forecast survives a refresh that moved nothing; choosing another B still asks. The first and third fail without the fix. |

## Results — 2026-10-08

- `bunx vitest run src/lib/merger src/routes/merge`: 44 passed.
- `bun run check`: zero errors, zero warnings.
