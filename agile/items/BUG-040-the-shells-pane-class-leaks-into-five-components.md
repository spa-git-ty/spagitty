<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-040 — The shell's pane class leaks into five components

**Status:** Fixed.
**Branch:** `bugfix/BUG-040-the-shells-pane-class-leaks-into-five-components`
**Screens:** Working copy (1C), Diff (1B), Stash (1G), Conflicts (1D), Farm
(1Q).
**Raised by:** the review of the screens not yet in the spatial language, from
screenshots of the Windows release build: the Farm drew a second bordered
panel inside the pane, and the Stash's diff sat in a rounded box of its own.

## Problem

FEAT-082 styled the one pane every screen sits in with a global rule in
`app.css`, `.pane`: a border, a lit top edge, the pane's corner, its shadow,
and — since FEAT-083 — the tokens that make a screen's bands step back.

`pane` is also the scoped class name of five components:
`changes/HunkPane.svelte`, `diff/DiffPane.svelte`,
`conflicts/SidePane.svelte`, `farm/components/ActivityDrawer.svelte`, and the
Farm screen's own sections. Svelte scopes a component's styles, not the
global stylesheet's, so each of these took the shell's rule as well as its own
and drew as a pane inside the pane: a second border and corner round Working
copy's hunks, the diff, both conflict sides and the farm's columns, and the
shell's shadow under each.

## Scope

- The global rule's class is `.window-pane`, used by the layout alone.

## Non-scope

- The components' own `.pane` classes, which are scoped and correct.

## Acceptance criteria

- The Farm, Working copy, Diff, Stash and Conflicts screens draw no border,
  corner or shadow round their inner columns that their own styles do not ask
  for.
- `app.css` has no global `.pane` rule, and only the layout uses
  `window-pane`.
