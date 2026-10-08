<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-058 — A conversation thread can be read in full

**Status:** Fixed — merged 2026-10-08.
**Screens:** 1R.
**Raised by:** the author, 2026-10-08: a long comment on the whole pull request was cut at three lines in the Conversation card, with no way to read the rest.

## Change

- A thread whose first comment runs past three lines, or which has replies, offers **Show more** under its card. It shows the first comment in full and every reply under it; **Show less** folds it back.
- A comment on the whole pull request has no line to jump to, so before this its text past three lines could not be read anywhere in the room.

## Acceptance criteria

- A clipped comment or a thread with replies shows Show more; a short comment with no replies does not.
- Show more shows the whole first comment and each reply with its author and time; Show less returns to three lines.
- Clicking the card still goes to the thread's line.
