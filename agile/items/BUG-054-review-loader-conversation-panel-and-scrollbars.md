<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-054 — Review's loader, its Conversation card, and scrollbars at rest

**Status:** Open — on its branch; the manual sweep is not yet run.
**Branch:** `bugfix/BUG-054-review-loader-conversation-panel-and-scrollbars`
**Screens:** 1R, all.
**Raised by:** the author, 2026-10-07, with a screenshot: Review showed only the small strands at the top while it read, never the main loader; the Conversation card's *Resolved* chip ran out past the card's edge, and the card could not be put away; and scrollbars should not show at rest, only while something scrolls.

## Change

- Review's inbox shows the main loader while its first list is read, and the room shows it while a pull request is fetched (it said *Fetching #N…* in a corner).
- The Conversation card's header wraps instead of spilling its chips, and the card clips to its own edge.
- A chevron in that header puts the card away; a tab down the room's right edge, with the open-thread count, brings it back. Remembered on this machine.
- Scrollbars are not drawn at rest. Whatever is scrolling shows its bar while it moves and for 0.9 s after (`is-scrolling`, set by one capture-phase listener in `ui/scrolling.ts`), and a bar shows under the pointer so it can still be grabbed.

## Acceptance criteria

- None of the above is visible in a release build.
