<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-041 — Automated test record

**Item:** [`agile/items/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow.md`](../items/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow.md)

## What was tested

`src/lib/ui/flat.test.ts`: the pinned header half paints no background at
rest, and paints the pane's colour while scrolled.

## Test command and output

On Windows 11: `bun run test` — Tests 3033 passed (3033).

## What is not covered automatically

The look. Checked in the Windows release build by injecting the rule: the
strip beside the card took the pane's tint.
