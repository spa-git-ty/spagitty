<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-084 — The other screens in the spatial language

**Status:** Open.
**Branch:** `feature/FEAT-084-the-other-screens-in-the-spatial-language`
**Screens:** Branches (1F), Tags (1N), Reflog (1M), Farm (1Q), Settings (1K),
File history (1O), and every screen's header and footer.
**Raised by:** the review of 2026-09-30, after FEAT-083: "a spatial pass for
screens other than Graph". FEAT-083 moved the Graph into the spatial language
and changed the rest only as far as two tokens reach.

## Problem

Screenshots of every screen in the Windows release build, beside the Graph:

- **Tables are still bands.** Branches, Tags and Reflog draw a rule under every
  row, a full-width highlight, and two or three buttons on every row all the
  time — "check out · edit message · delete" down all eighteen tags, "branch
  here · check out · reset here" down all 128 reflog entries, and "Branch from
  it · Delete" down a thousand remote branches. It is the crowding TASK-046
  took out of the file lists.
- **Their rows are a size above the rest.** The rows inherit the body's
  `--fs-ui`, the size BUG-036 took off the Graph's messages; a tag's message
  was larger again than a lightweight tag's summary beside it.
- **A tag's commit ran into its message.** Name, kind and commit could not
  shrink inside their 260px column, so a long name pushed the short id over
  the message ("f6c9aad" + "additional…").
- **Every header and footer is ruled.** FEAT-083 took their fill away; the
  hairline stayed, a line across one surface above and below every screen.
- **The branch table's column dividers are always drawn,** where the Graph's
  show only under the pointer.
- **Odd ones out.** The Farm names itself at body size where every other screen
  uses the title size; its starter is a card holding a grid of four more cards
  and three boxed checks. Settings' Identity Profiles heading is a size above
  and bold beside every other section's. File history's prompt sat at the
  pane's left edge in a flat box, because the page did not take the pane's
  width, with an input restyled away from every other field.

## Change

- **Rows are lines** on Branches, Tags and Reflog: the lists' type size, no
  rule, a rounded highlight inset from the pane's edges, and the actions shown
  on the row being hovered or holding focus.
- **The tag's cell holds its column**: the name gives way with an ellipsis.
- **`--band-rule`**, transparent inside the pane, draws every screen's header
  and footer rule; outside it each keeps the old hairline as a fallback.
- **Column dividers on Branches show while the header is hovered.**
- **The Farm's title** is the title size; the goal is a glass card; the steps
  and checks are lines, not boxes.
- **Identity Profiles' heading** is the other sections'.
- **File history's prompt** is centred in the pane, a glass card, with the
  standard field.

## Non-scope

- What any control does, and the copy.
- The Graph, Working copy, Stash and Diff, which FEAT-083 and TASK-046 covered.
- The screens' structure — no new headers, panes or navigation.

## Acceptance criteria

- On Branches, Tags and Reflog no row carries a rule or shows its buttons at
  rest; hovering or tabbing into a row shows them; rows are the lists' size.
- A long tag name ends in an ellipsis and never overlaps the message.
- No screen draws a rule under its header or over its footer inside the pane.
- The Farm's title matches the other screens'; its starter has one card.
- File history's prompt is centred in the pane.
