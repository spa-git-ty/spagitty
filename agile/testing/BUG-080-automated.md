<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-080 — Automated test record

**Item:** [BUG-080](../items/BUG-080-the-review-pill-runs-out-of-a-narrow-diff.md)

There is no automated test of this. It is style rules, and the test environment does not lay a page out, so a test could only read the rules back. What stands in for one is the measurement in the item: a page with the room's rules and the pill's, old and new, drawn by headless Chrome 155 at three widths, reporting each box's place. With the old rules the pill's buttons were outside it; with the new ones none is, and the diff is 560 wide.

The review room's and the resolver's mounted tests pass unchanged, so both pills still have every control they had.

## Results — 2026-10-12

- `bunx vitest run`: 190 files, 4136 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run build`: succeeded.
