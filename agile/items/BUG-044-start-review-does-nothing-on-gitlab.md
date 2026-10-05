<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-044 — Start review does nothing on GitLab

**Status:** Fixed.
**Branch:** `bugfix/BUG-044-start-review-does-nothing-on-gitlab`
**Screens:** Review (1R).
**Raised by:** the author, testing Review on a self-hosted GitLab: "when i
press start review it do nothing".

## Problem

GitLab names the project on every merge request it lists (`references.full`,
FEAT-088), the open repository's own list included. GitHub's list does not.
`review.keyOf` read a named project as a row from *All my repos* and took that
scope's host, which is only known once *All my repos* has been read. Under
*This repo* it was null, so there was no key, and `review.open` returned
without opening the room or saying why.

The same missing key left every GitLab card's saved progress unread.

A failure inside `review.open` — a clone lookup that throws, a repository that
will not open — was not caught either, so it was as silent.

## Scope

- A row of the open repository is keyed on the open repository's host,
  whether or not it names its project.
- A failure to open a review is said, the way *Open in worktree* says one.
- No changelog entry: Review is unreleased, and its *Added* entry already
  describes it working.

## Acceptance criteria

- Under *This repo* on GitLab, Start review opens the review room.
- A thrown failure while opening shows "The review could not be opened" with
  the cause, and the button is usable again.
