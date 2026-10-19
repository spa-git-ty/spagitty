<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-148 — Arrange Home's cards in Settings

**Status:** Done — on `release/3.3.0`; unmerged. Covered by Rust and component tests; not looked at in the running application (the author is checking it).
**Screens:** Settings › Home, 2A
**Raised by:** the author, 2026-10-19: "in settings make home cards arrangable".

## Change

Home (FEAT-140) drew its nine cards in a fixed layout. Settings gains a *Home*
section, after *Rail*, where each card can be shown or hidden and moved, the
way the Rail's screens are (FEAT-134).

## Acceptance criteria

- Settings › Home lists every card in Home's order, each with Show and Hide and
  a grip; Alt with ↑ and ↓ moves the focused row.
- Home draws the shown cards in that order. *Needs you* keeps its width: two
  columns and two rows where there are three columns, the whole row below that.
  The grid packs densely, so any order fills it.
- Reset brings back every card in the designed order.
- The choice is stored in `settings.json` as `home: { hidden, order }`, by card
  id. A card id this build does not know stays in the file; a card the stored
  order does not name goes after the card it follows in the design. A value
  this build cannot read costs only the layout.

## What was built

- `home/layout.ts`: the cards, their ids and labels, and the order arithmetic
  (`inHomeOrder`, `shownCards`, `moveCardBy`, `moveCardOnto`).
- `settings/HomeSection.svelte`, with a preview of the grid.
- `HomeScreen.svelte` draws one dense grid from `shownCards`; `ActivityCards`
  draws one card at a time (`card`).
- `src-tauri/src/settings.rs`: `HomeLayout`, read leniently.
