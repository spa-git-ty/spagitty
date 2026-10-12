<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-079 — Automated test record

**Item:** [BUG-079](../items/BUG-079-all-resolved-is-drawn-as-a-button.md)

| Suite | Cases |
| --- | --- |
| `src/routes/merge/resolve.test.ts` | Mounted, in Merger: with conflicts left the pill has the **Next unresolved** button and no status. With all four resolved it has one status, a `span`, that reads *All resolved*, and no button named *All resolved* or *Next unresolved*. |

The tests that step through the pill with **Next unresolved** pass unchanged.

## Results — 2026-10-12

- `bunx vitest run`: 190 files, 4136 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run build`: succeeded.
