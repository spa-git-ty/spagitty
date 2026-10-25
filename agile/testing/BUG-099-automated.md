<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-099 — Automated test record

**Item:** [BUG-099](../items/BUG-099-the-hooks-side-panel-cards-do-nothing.md)

- `src/lib/hooks/builder.test.ts`: with Husky managing the hooks, a catalogue card is enabled, a click asks to import, and agreeing imports and adds the block.

## Results — 2026-10-25

- `bunx vitest run src/lib/hooks`: 25 passed.
