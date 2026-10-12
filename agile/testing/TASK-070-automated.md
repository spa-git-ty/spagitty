<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-070 — Automated test record

**Item:** [TASK-070](../items/TASK-070-the-plans-file-list-scrolls-in-a-card-the-height-of-its-neighbours.md)

There is no automated test of this. It is two style rules, and the test environment does not lay a page out, so a test could only read the rule back. What stands in for one is the measurement in the item and the first and third rows of the sweep, taken from the built page in headless Chrome.

The Merger page's mounted tests (`src/routes/merge/`) pass unchanged, so the plan still draws its three cards and its files.

## Results — 2026-10-12

- `bunx vitest run`: 189 files, 4074 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run build`: succeeded.
