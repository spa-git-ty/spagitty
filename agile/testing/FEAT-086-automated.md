<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-086 — Automated test record

**Item:** [`agile/items/FEAT-086-the-pomodoro-theme.md`](../items/FEAT-086-the-pomodoro-theme.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/themes.test.ts` | Nine families, Giorno/Notte first; both variants pass every contrast rule. |
| `src/lib/theme.test.ts` | A stored `catppuccin` moves to Pomodoro once; Catppuccin chosen after the move stays; other families are left alone. |
| `src/lib/settings/sections.test.ts` | Appearance lists Pomodoro first and marks it in use. |

## Test command and output

On Windows 11: `bun run test` — Tests 3024 passed (3024).

## What is not covered automatically

How it looks. Checked in the Windows release build: see the sweep.
