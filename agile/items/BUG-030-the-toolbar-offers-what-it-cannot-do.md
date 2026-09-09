<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-030 — The toolbar offers what it cannot do, and hides what it can

**Status:** Fixed.
**Screen:** chrome (the toolbar, on every screen).
**Raised by:** a UI review: "clicking a prominent control does nothing; a
tooltip does not repair that loss of trust".

## Problem

**Two buttons that did nothing.** `Toolbar.svelte` opened with a group holding
Undo and Redo. Neither had a handler, neither had `disabled`, neither had
`aria-disabled`. Both carried `title: 'Not built yet'` and nothing else — so
they rendered as ordinary buttons in the most prominent row in the application,
took the pointer, took focus, announced themselves to a screen reader as
buttons, and did nothing at all when pressed.

A tooltip does not fix that. It is read *after* the click, by somebody using a
pointer, on hover, if they wait. To a keyboard user it is not there at all: the
control is in the tab order, answers Enter, and produces silence.

Worse, `chrome.test.ts` asserted the tooltip was attached — a test written to
describe the code rather than the requirement, so the defect had a passing test
guarding it.

**Two choices that were effectively hidden.** `pull()` takes three modes and
`fetch` takes a remote. Both alternatives existed and both were reachable only
by right-clicking the button, via `oncontextmenu`. That is undiscoverable with a
pointer — the only hint was the words "right-click for how" inside the same
tooltip — and **unreachable without one**: a `contextmenu` handler has no
keyboard path, so the fast-forward-only default was the only pull a keyboard
user could ever perform. Both menus also opened *at the pointer*, which is a
position that does not exist for a keyboard.

**Three answers to which key it is.** The palette formatted the modifier from
`navigator.platform`, the Appearance section wrote `Ctrl` into its markup on
every platform including macOS, and the title bar once carried a `⌘K` chip: a
macOS key name, on Linux, for a shortcut that was `⌘F`.

## Change

- **Undo and Redo are gone**, not disabled. A permanently dead control is still
  a claim that the feature is nearly here, and Spagitty's recovery story is not
  an undo stack — it is the Reflog screen, which is operation-specific, already
  built, and reachable from the rail and the palette. A general Undo needs a
  model of what each git operation reverses; inventing one to fill a gap in a
  toolbar is the wrong reason to design it. Three groups become two.
- **Pull and Fetch are split buttons.** The main half runs the safe default; a
  caret beside it opens the alternatives. The caret is a real `<button>` with
  `aria-haspopup="menu"`, a live `aria-expanded` and an `aria-label`, so it is
  in the tab order, answers Enter and Space, and is announced as a menu button
  rather than as "▾". `ArrowDown` on the main button opens the same menu, which
  is the convention for a menu button.
- **Both menus open under their control**, aligned to its left edge, the way
  the branch switcher already does (FEAT-045) — so a keyboard activation and a
  click put the list in the same place.
- **The right-click path is kept.** It was the only way in, and removing it
  would break the habit of everybody who found it.
- **The tooltips stop describing the interaction.** "Fetch every remote,
  pruning — right-click for one" said where the feature was hidden; the caret
  says it instead, and the tooltip says what the button does.
- **`src/lib/platform.ts`** (new) is the one place that answers "is this a
  Mac". `userAgentData.platform` first, `navigator.platform` next,
  the user agent string last — because the newest API is absent from WebKit,
  which is the engine Spagitty actually ships on, and the oldest is deprecated
  everywhere. `mod()`, `alt()` and `shortcut()` compose key names from it. The
  palette and the Appearance section both use it.

## Acceptance criteria

- No control on the toolbar carries a "not built" title, and none is inert.
- Every alternative to Pull and Fetch can be reached with the keyboard alone,
  and is visible without hovering anything.
- The caret reports its expanded state and has a name that is not a glyph.
- Shortcut labels read `⌘F` on macOS and `Ctrl+F` elsewhere, from one function.

## Non-scope

- **Building undo.** Explicitly refused above.
- **The rest of the toolbar's layout.** The chrome's height and the grouping of
  destinations are a separate item.

## Dependencies

FEAT-018 built the fetch-a-single-remote menu that this makes reachable.
FEAT-022 built Fetch and Push. FEAT-045 established anchoring a menu under its
control. BUG-018 established that a second press closes a menu.
