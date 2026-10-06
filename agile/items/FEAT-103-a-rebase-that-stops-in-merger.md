<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-103 — A rebase that stops, in Merger

**Status:** Open — built and merged into `main`; the manual sweep is not yet run.
**Branch:** `feature/FEAT-103-a-rebase-that-stops-in-merger`
**Screens:** 1S.
**Raised by:** the author, 2026-10-06, as slice 4 of `design_handoff_merger/`.

## Problem

A rebase applies one commit at a time and can stop on each. FEAT-101 could
only land one that never stopped, and undid any that did.

## Change

- **Rebase, then fast-forward runs in a worktree of its own**, under the git
  directory, detached at the source's tip: `rebase --onto <target> <base>`
  with diff3 markers. Neither branch moves and nothing checked out is touched
  while it runs or while it is stopped. The worktree is named after the merge,
  so leaving the screen and coming back finds the same rebase where it stopped.
- **Each stop is resolved with the same columns** (FEAT-102). The header says
  *Rebasing <source> onto <target> · commit n of N* and names the commit;
  git's ours and theirs are put back as A and B — swapping the markers when B
  receives — so the columns read A | B whichever branch lands. The replayed
  commit is named at the foot of its side.
- **Continue** settles every file of the stop and carries on (a commit whose
  changes were all resolved away is skipped, git's own answer); **Skip this
  commit** drops it; **Abort** undoes the rebase and removes the worktree,
  asking first.
- **Finish the rebase**: once every commit is replayed the commit dialog opens;
  its button moves the receiving branch to the replayed commits — `update-ref`
  against the tip read, or `merge --ff-only` where it is checked out — and the
  worktree is removed. Each commit keeps its own message.
- With nothing in the way, Merge now runs the same replay and goes straight to
  the dialog.

## Acceptance criteria

- The source branch is never moved; nothing moves until Finish the rebase.
- Abort at any point leaves the refs, the working tree and the worktree list
  as they were (`replay::tests::aborting_leaves_the_repository_as_it_was`).
- A stop is resolved with every choice FEAT-102 offers.
