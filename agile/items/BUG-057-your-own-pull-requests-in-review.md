<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-057 — Review lists your own pull requests

**Status:** Fixed — merged 2026-10-08.
**Screens:** 1R.
**Raised by:** the author, 2026-10-08: Review said *Nothing to review* while Pull requests listed open pull requests, each of which opened in the review room.

## Change

- The inbox left out every pull request the signed-in account opened. On a repository where all open pull requests are the reader's own, it showed nothing.
- They are now a fourth group, **Yours · you opened these**, after Needs you, Back with you and Open on this repo. A requested review on your own pull request does not put it in Needs you.

## Acceptance criteria

- With only your own pull requests open, Review lists them and does not say *Nothing to review*.
- Your own pull requests come last, under Yours; the other groups are unchanged.
- *Nothing to review* shows only when no pull request is open.
