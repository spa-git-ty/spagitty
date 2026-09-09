<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-080 — Follow Omarchy

**Status:** Backlog.
**Screen:** Settings → Appearance, and every screen that reads a colour.
**Raised by:** the author, on a machine running Omarchy: Spagitty should be able
to take its colours from the desktop it is running on.

## Problem

Spagitty carries eight palette families and no way to say "whatever the desktop
is using". On Omarchy the desktop's palette is a fact on disk that every other
application on the machine already follows — the terminal, the editor, the bar —
and Spagitty is the window that does not match.

## Sketch

A third theme **source** beside `manual` and `system` (BUG-031 built that
distinction): `omarchy`, offered when Omarchy is detected on Linux. It reads the
**active** palette rather than assuming a named theme, because Omarchy has no
one universal colour scheme — the machine this was raised on is running
`sushi-dark-palette`, not Tokyo Night.

The palette is read in Rust and returned as a typed, validated response; the
frontend never reads a path. Detection is capability-based rather than
inferred from `XDG_CURRENT_DESKTOP`, watching survives the directory
replacement `omarchy-theme-set` performs, and a literal copy of the desktop's
values is **not** what is applied — measured on the local palette, the
foreground/background pair is 17.22:1 but raw `muted` on `background` is 2.75:1,
so the derived app roles are adjusted for readability while keeping the
desktop's character.

Non-Omarchy platforms must be unaffected, and choosing a family must remain an
opt-out that sticks.

## Dependencies

BUG-031 separated the theme's *source* from its resolved mode and made the
first-paint cache hold resolved properties rather than a family name. Both are
prerequisites.
