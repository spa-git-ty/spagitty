<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-057 — Automated test record

**Item:** [`agile/items/TASK-057-the-log-screen-in-the-house-style.md`](../items/TASK-057-the-log-screen-in-the-house-style.md)

| Suite | Cases |
| --- | --- |
| `src/lib/search/screen.test.ts` | The head and the before-anything state; the count in the status pill and a row per result; the narrowest filter when nothing matched. |
| `src/lib/search/panes.test.ts`, `store.test.ts` | Unchanged and passing: the fields, chips, rows, detail and blame behave as before. |

Seen in a dev preview (headless Chrome, dark) with eight seeded results and an opened commit.
