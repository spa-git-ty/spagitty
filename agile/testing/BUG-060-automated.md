<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-060 — Automated test record

**Item:** [BUG-060](../items/BUG-060-merger-forecast-lost-on-first-open.md)

| Suite | Cases |
| --- | --- |
| `src/lib/merger/store.test.ts` | Three refreshes before the forecast answers still let it land, with one request; a moved tip asks again and only the newer answer is kept; a settled forecast survives a refresh that moved nothing; choosing another B still asks; a failed forecast is asked again on the next refresh rather than kept as the answer. The first and third fail without the fix, and the retry case fails without the review follow-up. |

## Results — 2026-10-08

- `bunx vitest run src/lib/merger src/routes/merge`: 45 passed (after the review follow-up).
- `bun run check`: zero errors, zero warnings.
