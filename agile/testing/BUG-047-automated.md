<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-047 — Automated test record

**Item:** [`agile/items/BUG-047-whole-file-scrolls-back-to-the-top.md`](../items/BUG-047-whole-file-scrolls-back-to-the-top.md)

## What was tested

`src/lib/ui/VirtualRows.test.ts`, *leaves an effect that asked for a row alone
when rows are measured (BUG-047)*: a harness asks for row 0 from an effect, the
reader scrolls to 10 000 px, and a row above the view is measured 30 px taller.
The effect runs once and the view stays at 10 030 px. Before the fix the effect
ran twice.

## Test command and output

On Windows 11: `bunx vitest run src/lib/ui/VirtualRows.test.ts src/lib/review
src/routes/review` — 85 passed.

## What is not covered automatically

Scrolling a real file in the release build. See the sweep.
