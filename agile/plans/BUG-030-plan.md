<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-030 — Plan

**Item:** [`agile/items/BUG-030-the-toolbar-offers-what-it-cannot-do.md`](../items/BUG-030-the-toolbar-offers-what-it-cannot-do.md)

## Approach

Both halves of this are the same mistake pointing in opposite directions: the
toolbar advertises what it cannot do and conceals what it can. So they are
fixed together, in one pass over one file, rather than as an accessibility
ticket and a discoverability ticket that would each half-touch the same markup.

The existing test is part of the defect and is rewritten first, before the
component, so that the requirement is stated before the code changes rather
than after.

## Decisions

- **Remove rather than disable.** The brief allowed either. A disabled control
  is a promise with a date on it, and there is no date; the honest version of
  "there is no undo" is that there is no undo button. The Reflog screen is the
  recovery model that exists, and it is already reachable three ways.
- **A split button, not a menu on the main button.** Pull's single click is
  fast-forward-only, which is the mode that cannot go wrong — that reasoning is
  already written beside `PULL_ITEMS` and this must not change it. Turning the
  whole button into a menu opener would cost the one-press safe path; a caret
  adds the choices without taking it.
- **The caret is its own `<button>`.** Two consequences follow for free: it is
  in the tab order, and it answers Enter and Space. Neither is true of a span
  with a click handler, and neither was ever true of `oncontextmenu`.
- **`ArrowDown` on the main button opens the menu too**, so the alternatives
  can be reached without tabbing past the action. It is the convention for a
  menu button and it costs four lines.
- **Anchor under the control, and capture the box *before* the await.**
  `openFetchMenu` reads the remotes before it can build its list; measuring
  afterwards would put the menu wherever the button had scrolled to by then.
- **The right-click handler stays.** Removing a working path because a better
  one now exists is a change that only costs people.
- **One platform module, three fallbacks.** `navigator.platform` alone is what
  the palette used, and it is deprecated with a decade of freeze proposals
  behind it. `userAgentData` is the replacement and is not in WebKit, which is
  the engine that actually renders Spagitty on Linux and macOS. Neither answers
  on its own, so both do, newest first, with the user agent string behind them.
- **`reset(source)` takes its source rather than reading a global**, so a test
  states the case it is testing instead of mutating something the next test
  inherits.
- **`mod()` carries its own separator.** `⌘F` and `Ctrl+F` — the separator is a
  property of the platform, not of the caller, so nothing that composes a
  shortcut has to know which one it got.

## Files

- `src/lib/chrome/Toolbar.svelte`
- `src/lib/platform.ts` (new), `src/lib/platform.test.ts` (new)
- `src/lib/palette/commands.ts`, `src/lib/settings/AppearanceSection.svelte`
- `src/lib/chrome/chrome.test.ts`
- `CHANGELOG.md`, `agile/` — this set and the index row

## Steps

1. Rewrite the toolbar assertions to state the requirement.
2. Remove Undo and Redo; collapse three groups to two.
3. The split button: markup, the two anchored openers, the styles.
4. `platform.ts`, its tests, and the two call sites that had their own answer.
5. Changelog, this set and the index row.

## Risks and rollback

- **The toolbar gets wider** by two carets and narrower by two buttons, so it
  nets narrower. The 900px breakpoint hides labels; the caret must survive
  that, and the rule is scoped to `.tool > span:last-child` so the caret's
  glyph — which is not a label — is not caught by it.
- **`aria-expanded` is derived from which menu is open**, keyed off the
  action's label. That is a string comparison inside the render, and it is the
  one thing here that would break silently if an action were renamed; the test
  asserts both carets report `false` at rest and the pull caret reports `true`
  once opened.
- Rollback is per file. Nothing here changes a git operation or any state that
  outlives a session.
