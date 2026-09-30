<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-034 — A transparent band round the window on Windows

**Status:** Fixed.
**Branch:** `feature/FEAT-083-the-content-in-the-spatial-language`
**Screens:** all — the window's own edge, on Windows.
**Raised by:** the author, running the native Windows build: "there's a
transparent surround around the app."

## Problem

On Windows the application draws its own window: a card with a rounded corner
and a shadow, inside a 10px margin for the shadow to fall into (FEAT-037). The
window is transparent so the margin can be, and so the desktop showed through
a band round the whole application — inside Windows 11's own frame, because an
undecorated window with `shadow: true` is already given a rounded corner and a
shadow by the system.

This is BUG-029 on the third platform. Linux stopped drawing the card there,
because the compositor frames the window; macOS stopped in TASK-042, because it
is decorated. Windows was left drawing its own on the grounds that an
undecorated window there had no frame. On Windows 11 it has one.

## Scope

- Windows draws no card: `decoratesItself` is false for `Windows NT` too, so
  the shell is `flush` — no margin, no drawn corner or shadow.
- `shadow: true` is said in the window manifest rather than left to Tauri's
  default, and mirrored in the Linux override the manifest test pins.

## Non-scope

- Windows 10, which does not round an undecorated window's corner. It gets a
  square window with the system's shadow, which is how its own applications
  look.
- An operating-system material behind the window (Mica).

## Acceptance criteria

- On Windows the application fills its window to the edge, with no transparent
  band; the corner and the shadow are the system's.
- A host nothing recognises still gets the drawn card.
