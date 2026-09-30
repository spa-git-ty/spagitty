<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-082 — A spatial shell

**Status:** Open — merged into `main` on 2026-09-30; the manual sweep is still owed.
**Branch:** `feature/FEAT-082-a-spatial-shell`
**Screens:** chrome — the title bar, the tabs, the toolbar, the rail and the
status strip — and the floating layers: menus, dialogs, the palette, toasts.
Every screen's *content* is untouched, the Graph above all.
**Raised by:** the author: "revamp the surrounding UI (except the graph area)
to follow the new design concept of spatial app design", and, asked how far,
"full glass".

## Problem

The shell is a stack of full-width bars around a screen: a title bar, a tab
row, a toolbar, the screen, a status strip, and a rail down the side. Each is a
strip cut out of one flat surface. TASK-041 made them quieter and thinner; they
are still five bands of chrome, 150 pixels of them before and after the work on
a window 800 high, and none of them reads as an object.

Spatial design, as visionOS draws it, has a different vocabulary: content lives
in a pane of glass floating over an environment, and the controls around it are
*ornaments* — small floating objects attached to the pane, not bars across it.
Navigation is a vertical pill of icons at the pane's side that shows its labels
when it is looked at. Actions are a pill below the pane.

## Change

**One pane, on an environment.** The window paints an environment — the
theme's ground, lit by soft washes of its own accent and lane colours — and the
screen sits on it in one pane with a large corner, a glass edge and a soft
shadow. The pane itself is opaque: it holds the graph, and nothing is blurred
behind it.

**One row above.** The title bar and the tab row become one row on the
environment: the open repositories as pills, `+`, and the window controls. The
program's name shows only when there are no tabs.

**The rail is an ornament.** A floating vertical pill of icons beside the
pane, with each count as a small badge on its icon. Hovering or focusing it
widens it over the pane to show the labels. There is nothing to collapse and
nothing to drag, so the collapse button and the rail's splitter go.

**The toolbar is an ornament.** The branch picker and the actions become one
floating pill centred below the pane. The repository's name leaves it — the
active tab says it — and so does the Settings gear, which the rail has.

**The status strip shares the ornament's row.** Identity and the repository's
state to its left; the counts and the licence to its right, the counts giving
way first on a narrow window.

**Glass is spent on what floats.** The ornaments, menus, dialogs, the palette
and toasts are glass: tinted, blurred, lit along the top edge. They are small
and few, so the blur is cheap even on the software path. The environment is
static, so a pane over it looks the same blurred or not, and nothing large is
blurred.

**Rounder.** Spatial controls are capsules and its surfaces are generously
rounded. Fields, buttons and chips step up; panes and floating layers step up
further.

## Non-scope

- Operating-system materials (Mica, Acrylic, vibrancy) behind the window. They
  need the running application on each platform to verify, and are their own
  item.
- The content of any screen: headers, tables, the graph, the diff.
- Reverting TASK-045's decisions about what is in the rail.

## Acceptance criteria

- With a repository open, the window has one row above the pane and one below
  it; no full-width bar remains.
- The rail is a floating pill of icons with count badges, and shows its labels
  on hover and on keyboard focus without moving the pane.
- The toolbar is a floating pill below the pane with the branch picker, Pull,
  Fetch, Push, Clone, Branch, Stash and Rebase, and the command log toggle when
  that setting is on.
- Identity, repository state, counts and licence are all still on screen at
  1280 pixels wide; at 900 the counts go first and the licence stays.
- The Graph screen's content renders exactly as before inside the pane.
- Backdrop blur is used only by the ornaments and the transient floating
  layers, never by the pane, the environment or a list row.
- Both themes, and every palette family, read correctly.
