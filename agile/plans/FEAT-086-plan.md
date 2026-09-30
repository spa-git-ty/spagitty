<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-086 — Plan

**Item:** [`agile/items/FEAT-086-the-pomodoro-theme.md`](../items/FEAT-086-the-pomodoro-theme.md)

## Approach

Two `Palette`s in `themes.ts`, `GIORNO` and `NOTTE`, with the values the
concepts page showed and the brand guide records. They go first in `FAMILIES`
and become `DEFAULT_FAMILY`, so Settings lists them first and a fresh install
starts on them. No token is new: the environment, glass and pane are derived
in `app.css` from `--accent`, `--lane-1`, `--lane-2` and `--panel`, so the
spatial shell follows the family by construction.

`app.css`'s `:root` and `:root[data-theme='dark']` boot values become the two
palettes, token for token, so the frame before JavaScript runs is already the
default family.

`theme.init()` moves a stored `catppuccin` to the default once, and writes
`spagitty.theme.pomodoro` so it never does again.

## Files

| File | Change |
| --- | --- |
| `src/lib/themes.ts` | `GIORNO`, `NOTTE`, the family, the default. |
| `src/lib/theme.svelte.ts` | The one-time move. |
| `src/app.css` | Boot values. |
| `src/lib/themes.test.ts`, `theme.test.ts`, `settings/sections.test.ts` | Nine families; the move; Pomodoro first. |

## Risks and rollback

- **Somebody who chose Catppuccin on purpose** finds Pomodoro after upgrading,
  once, and chooses Catppuccin again; it then stays.
- Rollback is a revert.
