<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-086 — The Pomodoro theme

**Status:** Open.
**Branch:** `feature/FEAT-086-the-pomodoro-theme`
**Screens:** all.
**Raised by:** the author, 2026-09-30, choosing the rebrand's scope: "also
current themes looks awful and non coherent they need to aligne with spatial
designe".

## Problem

Spagitty had no theme of its own. Its default was Catppuccin, chosen because
its peach sat next to the old amber mark; the other seven families are
published palettes, each built around its own hue. None of them was drawn for
the spatial shell — an environment lit by the accent and two lane colours, a
warm pane, glass ornaments — and none of them shares a colour with the new
mark (FEAT-085).

## Change

- **Pomodoro**, a family of Spagitty's own, first in the list and the default:
  *Giorno* (light) and *Notte* (dark). Tomato accent; basil, saffron,
  aubergine and sky for the lanes; warm cream and warm charcoal grounds; a
  crimson danger so a destructive button is never read as the accent. The
  environment, the pane and the ornaments already mix from these tokens.
- **The first paint** is Pomodoro: `app.css` carries its two palettes as the
  boot values.
- **Existing installs move once.** Catppuccin was the default and the family
  is written on every mode change, so a stored `catppuccin` is moved to
  Pomodoro once; a marker keeps a later, deliberate choice of Catppuccin. Any
  other stored family is left alone.

## Non-scope

- Removing or redrawing the other eight families. Whether they stay, as
  published palettes, is the author's call.

## Acceptance criteria

- A fresh install, and an install that was on Catppuccin, opens on Pomodoro.
- Both variants meet every readability rule `themes.test.ts` holds a palette
  to.
- The environment, pane, rail and toolbar read as one warm scheme in both
  modes.
