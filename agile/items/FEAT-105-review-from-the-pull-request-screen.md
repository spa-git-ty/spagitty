<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-105 — Review from the pull request screen

**Status:** Open — on its branch; the manual sweep is not yet run.
**Branch:** `feature/FEAT-105-review-from-the-pull-request-screen`
**Screens:** 1H, 1R.
**Raised by:** the author, 2026-10-07: in Pull requests a pull request could be read as files and changes, but not reviewed or commented on, though Review (1R, FEAT-087–FEAT-093) now does both.

## Change

- The open pull request's header carries **Review**, first among its actions: it opens that pull request in the review room, where comments are written on lines and ranges, threads answered and resolved, and a review finished with a verdict.
- The author's footer (developer view), which had no way to answer, offers **Reply in Review** beside its count of open threads.
- The screen's own inline drafts and Publish Review stay as they were.

## Acceptance criteria

- From any open pull request, one click reaches a place to comment and to publish a review.
