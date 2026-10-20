<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-096 — Automated test record

**Item:** [BUG-096](../items/BUG-096-the-working-copy-s-hunks-are-not-coloured.md)

- `src/lib/changes/panes.test.ts`: a Rust hunk's added line has `fn` as a keyword, the changed `2` as a number marked as a changed word, and its text unchanged.

## Results — 2026-10-20

- `bunx vitest run src/lib/changes`: 96 passed.
