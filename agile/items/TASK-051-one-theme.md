<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-051 — One theme

**Status:** Open.
**Branch:** `task/TASK-051-one-theme`
**Screens:** Settings → Appearance; every screen's colours.
**Raised by:** the author, 2026-09-30. Asked whether the eight published
palette families should stay beside Pomodoro or go: "remove them".

## Problem

Nine theme families: Pomodoro (FEAT-086), drawn from the brand for the spatial
shell, and eight published palettes — Catppuccin, Dracula, Tokyo Night,
Gruvbox, Nord, Rosé Pine, Solarized, Everforest — none of which was drawn for
it or shares a colour with the mark. The author had called them "awful and non
coherent": chosen, they made Spagitty look like nine applications.

## Change

- **One family.** `themes.ts` carries Pomodoro alone, Giorno and Notte. The
  family shape stays, with one entry, so the theme store and following the
  desktop's palette under Omarchy keep their one way of asking for colours.
- **Appearance** offers Light, Dark, Follow system and, under Omarchy, Follow
  Omarchy; the grid of families is gone. Light and Dark are how one leaves the
  desktop's palette, as they already were.
- **A stored family that no longer exists** is not a family and falls to
  Pomodoro, like anything unreadable. FEAT-086's one-time move from Catppuccin
  and its marker key are no longer needed and are gone.
- The README, `docs/screens.md` and the changelog say one theme.

## Non-scope

- Following the desktop's own palette (FEAT-080), which stays.
- The Pomodoro palettes themselves.

## Acceptance criteria

- Appearance offers no family; the application is Giorno or Notte, or the
  desktop's palette under Omarchy.
- An install stored on any of the eight removed families opens on Pomodoro in
  the mode it had.
- Both Pomodoro variants still pass every contrast rule `themes.test.ts` holds.
