<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-089 — A pull request read from disk

**Status:** Open — built on `feature/FEAT-089-a-pull-request-read-from-disk`, not yet merged.
**Branch:** `feature/FEAT-089-a-pull-request-read-from-disk`
**Screens:** 1R.
**Raised by:** the author, 2026-10-04, as slice 2 of the Review handoff
(`design_handoff_review/README.md`): "Whole file needs local blobs".

## Problem

Spagitty reads a pull request's changes from the host's patch. A patch has the
changed lines and three lines either side, and nothing else: no whole file, no
way to unfold what lies between two hunks, and no checkout to build and run.
The Review room needs all three.

## Change

- **The head is fetched into the repository.** Hosts publish every pull
  request's head under a ref of their own — `refs/pull/N/head` on GitHub,
  `refs/merge-requests/N/head` on GitLab. It is fetched into
  `refs/spagitty/pull/N`, which no branch list, graph or tag list reads, and
  the target branch into its ordinary remote-tracking ref. No tags; no
  `FETCH_HEAD`. A head already fetched and equal to the one the host reports
  is not fetched again.
- **The diff is the host's diff, made locally**: from where the head left the
  target (their merge base) to the head, so changes that reached the target
  since are not shown as the pull request's.
- **A file is read whole**: every line of the new version with the removed
  lines in place, and each side's blob id. The room cuts its changed parts and
  its folds from these lines, so unfolding asks the backend nothing more; and
  a viewed tick is kept against the new blob, so a file the author changes
  later reads as unviewed.
- **A renamed file says where it was** (`oldPath` on each changed file), so
  its old side is read from the old path.
- **Open in worktree**, on the inbox's preview and in the room's header: the
  head goes in a worktree beside the repository, `<name>-pr-<N>`, detached.
  Opened again after the author pushes, the same worktree moves to the new
  head — unless it has changes of its own, which git refuses to throw away.
  A toast says where it is.

## Non-scope

- Bitbucket, which publishes no pull request ref.
- The room that reads these files: its own item.

## Acceptance criteria

- A pull request on GitHub or GitLab is fetched without changing any branch,
  and its file list matches the host's.
- A whole file is every line of the head with the removed lines in place.
- Open in worktree makes `<name>-pr-<N>` at the head, and moves it when the
  head moves.
