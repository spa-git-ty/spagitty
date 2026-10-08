<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-062 — Automated test record

**Item:** [BUG-062](../items/BUG-062-after-a-merge-another-pull-request-opens-empty.md)

| Suite | Cases |
| --- | --- |
| `src/lib/requests/workspace.test.ts` | With #412 and #2 open and the workspace on #412, a successful merge returns to the list rather than showing #2; the same after a close. A workspace opened before the list arrives reads the files and commits of the pull request it lands on. Through *Merge* and *Confirm Merge*, the notice reads *#412 merged* and the view is the list. All four fail without the fix. |

## Results — 2026-10-08

- `bunx vitest run`: 3,554 passed across 170 files.
- `bun run check`: zero errors, zero warnings.
