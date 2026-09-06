<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-036 — Use the current shared amendments book

**Status:** Open — local update and validation complete; not yet submitted or merged.
**Branch:** `task/TASK-036-sync-amendments`
**Screens:** —.

## Problem and scope

The author requested that this repository use the current book at
`/home/maxmya/dev/agents/docs/AMENDMENTS.md`. The pointer already named that
file, but its ratification dates and description of Amendments 19 and 20 were
outdated. The existing starter `AGENTS.md` did not direct agents to the book.

Update the pointer and add canonical-book instructions to `AGENTS.md`, preserving
its existing rules. Keep superseded pointer claims clearly labelled as history.
The canonical book itself, skill files, application behavior, and the separate
farm reliability implementation are outside this item.

## Acceptance criteria

- Both agent entry points direct readers to the same existing canonical file.
- The pointer accurately records Amendments 0 through 20, the 2026-08-28
  ratification, and the current subjects of Amendments 19 and 20.
- No independent copy or modification of the frozen book is introduced.
- The working record checks, frontend tests, and frontend build pass.
- Changes remain on the item's branch until reviewed and merged through a PR.

## Dependencies

None. This documentation update can precede the farm reliability work.
