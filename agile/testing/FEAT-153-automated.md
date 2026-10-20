<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-153 — Automated test record

**Item:** [FEAT-153](../items/FEAT-153-blame-on-click-can-be-turned-off.md)

- `src/lib/files/files.test.ts`: with the toggle on a click opens the peek; turned off, the open peek closes, a click moves the cursor and opens nothing, and Enter still opens it. The choice is read back after the store is cleared and the repository entered again.

## Results — 2026-10-20

- `bunx vitest run src/lib/files`: 160 passed.
