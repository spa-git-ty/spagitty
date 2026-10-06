<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-101 — Merge without conflicts

**Status:** Open — built and merged into `main`; the manual sweep is not yet run.
**Branch:** `feature/FEAT-101-merge-without-conflicts`
**Screens:** 1S.
**Raised by:** the author, 2026-10-06, as slice 2 of `design_handoff_merger/`:
Merge now, for every direction and strategy, into a branch that may not be
checked out, including Squash.

## Problem

Merger (FEAT-100) shows what a merge would do and cannot do it. And a branch
that is not checked out cannot be merged into at all without checking it out,
which moves the working tree under whatever the person was doing.

## Change

- **Merge now** opens the commit dialog when the dry run found nothing in
  conflict (or for Fast-forward only, which never conflicts). It names what
  will be written, takes the message (prefilled `Merge branch '<source>' into
  <target>`, or `Squash <source> into <target>`), and its button names the
  strategy: Create merge commit, Commit the squash, Finish the rebase,
  Fast-forward. Back writes nothing.
- **Nothing is checked out.** A merge or squash is the dry run's tree,
  committed with `commit-tree` (which signs where `commit.gpgSign` says to).
  A rebase replays the source's commits in a scratch worktree under the git
  directory; the source branch is not moved. A fast-forward is the source's
  tip.
- **The branch moves last, and only if it is where the plan read it.** Not
  checked out anywhere: `update-ref` against the tip that was read. Checked
  out here or in another worktree: `merge --ff-only` in that worktree, which
  brings its files along and refuses rather than overwrite uncommitted work.
  A new branch is created at the result. Either branch having moved since the
  forecast is refused.
- **Done**: *<target> now includes <source>*, what was written, Open in Graph
  and Merge another.
- The landing path takes a resolution per conflicted path — text, or one side
  whole, which for a side that deleted the file deletes it — and refuses text
  that still has markers in it. FEAT-102 supplies them.

## Non-scope

Resolving conflicts on screen (FEAT-102); a rebase that stops (FEAT-103: here
a rebase that hits a conflict is undone with its worktree and says so). Hooks:
`commit-tree` and `update-ref` run none, where `git merge` would run
`pre-merge-commit` and `commit-msg`.

## Acceptance criteria

- Merging into a branch that is not checked out never changes the checked-out
  branch, its working tree or the index
  (`land::tests::a_merge_into_a_branch_that_is_not_checked_out_leaves_the_working_tree_alone`).
- Merge, squash, rebase and fast-forward each land into A, into B and into a
  new branch.
- A branch that moved since the plan was read is refused, not overwritten.
