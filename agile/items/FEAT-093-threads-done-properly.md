<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-093 — Threads done properly

**Status:** Open — built on `feature/FEAT-093-threads-done-properly`, not yet merged.
**Branch:** `feature/FEAT-093-threads-done-properly`
**Screens:** 1R.
**Raised by:** the author, 2026-10-04, as slice 5 of the Review handoff
(`design_handoff_review/README.md`, "Threads done properly").

## Problem

The review room could read threads but not take part in them. Comments could
not cover a range, a thread could not be resolved — GitHub's comments were
read as never resolved — comments on the pull request as a whole were neither
read nor written, and the Pull requests screen keeps unsent comments in the
browser's storage, where a reinstall or another profile loses them.

## Change

- **A `+` on every line** opens a box under it; **shift-clicking a line
  number** stretches it over a range — across both sides when the range
  covers removed and added lines. The lines covered are tinted while writing.
- **What is written is pending** — warm-tinted, under its lines, *Pending ·
  goes out with Finish review* — and kept in the pull request's record in
  Spagitty's application data, against the head it was written on. It
  survives a restart. One written before the author's last push is not placed
  on lines that may have moved: the Conversation card lists it apart, to be
  deleted or rewritten.
- **Finish review · N** in the header opens a card: Comment, Approve or
  Request changes, the words for the pull request as a whole, and how many
  line comments go with it. Sending submits them as one review; until then
  nothing reaches the host. A comment or changes asked for with nothing
  written cannot be sent.
- **Threads from the host** take a **reply**, sent at once, and are
  **resolved or reopened** on the host — GitHub's `resolveReviewThread` and
  `unresolveReviewThread`, GitLab's `PUT …/discussions/:id` with `resolved`.
  The change shows at once and goes back, with the reason, if the host
  refuses. GitHub's resolved state is now read, through the review threads.
- **Comments on the pull request as a whole** — GitHub's conversation, GitLab's
  notes off any line — are listed in the Conversation card as *whole PR*. At
  the card's foot, the box for your own goes out with Finish review.
- **Ranges reach GitLab placed**: each end carries both versions' counters, so
  a range becomes the `line_range` GitLab's spec asks for.

## Non-scope

- The Pull requests screen, whose comments and drafts stay as they are; it
  reads the same line comments it did.
- Editing a comment already sent, and reactions.

## Acceptance criteria

- A pending comment survives a restart and is sent only by Finish review.
- Resolving a thread in Spagitty resolves it on the host, and the reverse
  shows after a refresh.
- A comment can cover a range of lines, on GitHub and on GitLab.
- Comments on the pull request as a whole are read, and one can be sent.
