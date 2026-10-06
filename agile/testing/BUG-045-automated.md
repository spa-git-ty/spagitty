<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-045 — Automated test record

**Item:** [`agile/items/BUG-045-the-name-leaves-the-title-bar.md`](../items/BUG-045-the-name-leaves-the-title-bar.md)

## What was tested

`src/lib/chrome/chrome.test.ts`, *carries the tabs as pills, and the name
beside them (FEAT-082, BUG-045)*: with a tab open, the tab is drawn and the
name is there. It failed before the fix.

## Test command and output

On Windows 11: `bunx vitest run src/lib/chrome` — 89 passed.

## What is not covered automatically

Where the name sits beside a long row of tabs. See the sweep.
