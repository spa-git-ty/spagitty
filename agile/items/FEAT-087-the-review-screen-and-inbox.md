<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-087 — The Review screen and its inbox

**Status:** Open — built on `feature/FEAT-087-the-review-screen-and-inbox`, not yet merged.
**Branch:** `feature/FEAT-087-the-review-screen-and-inbox`
**Screens:** 1R (new), chrome (the rail).
**Raised by:** the author, 2026-10-04: reviewing merge and pull requests on
GitHub or GitLab is cluttered, hurts to read as a dyslexic reviewer, and major
review points get missed. "Don't remove current pr screen just build new review
one." The design is `design_handoff_review/` (Claude Design, the same date);
this is its slice 1.

## Problem

Pull requests (1H) is good for browsing, creating and merging, and not enough
for reviewing. There is nowhere that says which pull requests are waiting on
your review, which ones came back to you after you commented, and how far you
got through each.

## Change

- **Review (1R)**, a new screen at `/review`, on the rail right after Pull
  requests with an eye for an icon. The rail draws a dot on it while a pull
  request in the open repository has you as a requested reviewer. Pull requests
  is unchanged.
- **The inbox**, in three groups: *Needs you* (you are a requested reviewer),
  *Back with you* (a thread you started has an answer), and *Open on this repo*
  (somebody else's, nobody asked you). Your own pull requests are left out —
  Pull requests is where you follow them. Each card: number, title in the
  reading face, author and age, the branch, a *conflict fixes* chip once a look
  has found one, open threads (or replies to you), one to three bars for size,
  and a progress bar — *not started*, *3 of 6 viewed*, or *changed since you
  looked* when the author pushed after your last look.
- **The preview card**, an inset card like Graph's commit detail: where the
  pull request goes, its description, *Before you start* (conflict fixes,
  threads, checks, your pending comments — only what is known), and *Start
  review* or *Continue review · 3 of 6 viewed*.
- **All my repos**: open pull requests anywhere on the host that involve you,
  each naming its repository. Opening one opens the clone Spagitty knows of it;
  with none, the card says *No clone of team/app here*.
- **Per pull request state**, kept as one JSON file under Spagitty's
  application data (`reviews/<host>/<owner…>/<name>/<number>.json`), written by
  temporary file and rename. It holds the viewed ticks, pending comments and
  what the last look learnt; the later slices fill it. Every path part from
  the webview is checked to be a plain name.
- **What the host sends, read once.** The GitHub list asks for the head commit
  and the review threads (first and last author of each) in the same GraphQL
  request; GitLab rows carry `sha`, `reviewers` and the project path. The list
  is read when a repository opens — so the dot is right before the screen is
  visited — and on Refresh, never on a timer.
- **The room opens** from the inbox with its header — back to Review, number,
  title, who wants to merge what into what, checks, host. Its body is a later
  slice.

## Non-scope

- The handoff's other slices, each its own item: the local checkout and diff,
  the room's files and diff, conflict fixes, threads, GitLab, and Settings ›
  Reading. GitLab's thread counts in the list are part of the GitLab work.
- Changing the Pull requests screen.

## Acceptance criteria

- The rail shows Review after Pull requests, with a dot while a review is asked
  of you in the open repository.
- The inbox groups pull requests as above, leaves out your own, and the
  preview follows the chosen card.
- A saved review shows its progress on the card and in the button.
- All my repos lists pull requests across repositories and opens the right
  clone, or says there is none.
- Errors are the host's own sentence, with the way to Settings → Accounts.
