<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-061 — Automated test record

**Item:** [BUG-061](../items/BUG-061-merger-shows-the-previous-pairs-forecast.md)

| Suite | Cases |
| --- | --- |
| `src/routes/merge/page.test.ts` | Choosing another B while its forecast is pending shows *Working out the merge…*, not the old pair's plan, and offers neither *Resolve 4 conflicts* nor *Merge now* until the new forecast lands (fails without the fix). Changing only the strategy keeps the plan without a loader or a second request. Two existing fixtures now name the sides as asked, as the backend does. |

## Results — 2026-10-08

- `bunx vitest run`: 3,552 passed across 170 files.
- `bun run check`: zero errors, zero warnings.
