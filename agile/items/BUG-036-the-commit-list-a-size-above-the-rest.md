<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-036 — The commit list, a size above the rest

**Status:** Fixed.
**Branch:** `bugfix/BUG-036-the-commit-list-a-size-above-the-rest`
**Screens:** Graph (1A).
**Raised by:** the author: "in graph view the commit message text size is not
aligned with the screen."

## Problem

The Graph's message, author and date cells set no type size, so they inherited
the body's `--fs-ui` (15.6px at 100%). Every other list — Working copy's files,
the stash, the diff's file list, the column headers above these very cells —
uses `--fs-secondary` (13.2px). At the author's text scale the messages were
drawn at about 24px beside 17px everywhere else: the history read as if it came
from another application.

## Scope

- The message and text cells take `--fs-secondary`.

## Non-scope

- The row pitch and the lanes, which are unchanged.

## Acceptance criteria

- Commit messages, authors and dates are the same size as the lists on the
  other screens; the rows, lanes and nodes do not move.
