<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-030 — Manual sweep

**Item:** [`agile/items/BUG-030-the-toolbar-offers-what-it-cannot-do.md`](../items/BUG-030-the-toolbar-offers-what-it-cannot-do.md)

**Run the keyboard tickets with the mouse physically unplugged, or hands off
it.** The defect being fixed is one a pointer user cannot experience: the three
pull modes were on a `contextmenu` handler, which has no keyboard path at all.
A sweep run with a hand resting on a mouse would pass without testing anything.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | Any repository open | Look at the toolbar | Pull, Fetch, Push, Clone, then Branch, Stash, Rebase. **No Undo, no Redo.** One divider, not two. | High | |
| SWEEP-002 | The same | Look at Pull and Fetch without hovering anything | Each is visibly one object in two parts, with a caret on the right. Somebody who has never used Spagitty can tell there are choices without being told. Before this, the only hint was a sentence inside a tooltip. | High | |
| SWEEP-003 | The same | Hover the main half, then the caret | Each half highlights on its own and neither moves. A two-part control whose halves lift independently reads as coming apart. | Medium | |
| SWEEP-004 | An upstream that cannot fast-forward | Tab from the branch switcher until focus reaches the Pull caret. Press Enter | The menu opens **under the button**, not at the last pointer position. Arrow to "Rebase my commits on top" and press Enter; the rebase runs. This is the operation a keyboard user could not previously perform at all. | High | |
| SWEEP-005 | Two or more remotes | Focus the Fetch button and press `ArrowDown` | The remote list opens, with "Every remote" first and each remote under it. Escape closes it and focus returns to the button. | High | |
| SWEEP-006 | A screen reader — Orca on Linux, VoiceOver on macOS | Move to each caret | It is announced as a menu button with a name — "How to pull", "What to fetch" — and its collapsed state. Not "▾, button". | High | |
| SWEEP-007 | A Mac | Open the command palette; then Settings → Appearance | Shortcuts read `⌘F`, `⌘+`, `⌘−`, `⌘0` — with no `+` between the glyph and the key. The Appearance note says `⌘`, not `Ctrl`. On Linux the same places say `Ctrl+F` and `Ctrl`. | High | |
| SWEEP-008 | Window narrowed below 900px | Look at the toolbar | Labels disappear; the carets do not. Pull and Fetch are still two-part controls, and nothing overlaps. | Medium | |
| SWEEP-009 | Any repository | Right-click Pull, then right-click Fetch | Both still open their menus, as they always did. The fix adds a path; it must not have moved one. | Medium | |
| SWEEP-010 | Reduced motion enabled in the desktop | Repeat SWEEP-003 | Nothing animates. | Low | |
| SWEEP-011 | Any repository | Press the caret, then press it again | The menu closes. A control that only opens is one you have to click elsewhere to escape (BUG-018). | Medium | |

## Negative paths this sweep deliberately covers

- **SWEEP-004 and SWEEP-005** are the whole bug. Every other ticket here would
  have passed before the change.
- **SWEEP-009** is the regression half: a working path was kept deliberately,
  and it would be easy to lose while adding the new one.
- **SWEEP-008** is where the change can break something it did not intend to —
  the narrow layout hides `.tool > span:last-child`, and the caret's glyph is
  also a span.

## What is being looked for that no ticket can spell out

Whether a person who has never used this application can tell, in one glance
and without hovering, that Pull has alternatives. That is the requirement; the
tickets above are the evidence for it.
