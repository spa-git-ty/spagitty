<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-057 — The Log screen in the house style

**Status:** Done — merged into `main`; the manual sweep is not yet run.
**Branch:** `task/TASK-057-the-log-screen-in-the-house-style`
**Screens:** 1I.
**Raised by:** the author, 2026-10-07: "log screen looks borrowed, not matching our design."

## Change

Log is laid out like Merger, Review and Rebase, with nothing it could do removed:

- The head every designed screen has — icon tile, title, one line — and a status pill: how many results, or that the walk is still going.
- The question on a card of its own: six labelled fields (Author, Message, Path, In the diff, Since, Until) as sunken rounded inputs in one row, Search beside them, and the applied filters as chips under them.
- Results on a card: each a two-line row — the author's face (the graph's portrait, picture where there is one), the subject and its labels, the author and when — with the short id as a pill. Empty and nothing-matched states are centred with a mark, not a line of text in a corner.
- The opened commit and Blame on cards of their own beside the results; the commit shows its author as a person, with face and address.

## Acceptance criteria

- The Log screen reads as part of the same application as Merger and Rebase.
