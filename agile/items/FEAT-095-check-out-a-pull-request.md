<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-095 — Check out a pull request

**Status:** Open — built on `feature/FEAT-095-check-out-a-pull-request`, not yet merged.
**Branch:** `feature/FEAT-095-check-out-a-pull-request`
**Screens:** 1R.
**Raised by:** the author, 2026-10-05: "open in working dir does not make
sense make it checkout this branch instead".

## Problem

FEAT-089's *Open in worktree* put a pull request's head, detached, in a folder
beside the repository. The author wants what a reviewer usually wants: the
pull request's branch checked out in the repository they have open.

## Scope

- **Check out branch** replaces *Open in worktree* on the inbox's preview and
  in the room's header. It fetches the head as before (FEAT-089), then checks
  it out:
  - under the pull request's own branch name when no branch here has that
    name — made at the head — or when the branch of that name is already at
    the head;
  - as `pr-N` otherwise: a branch of yours by that name somewhere else, or a
    name that is the target's (a fork's `main` into `main`). `pr-N` is
    Spagitty's and only ever moves forward; one with commits the pull request
    does not have is refused, not moved.
- A new branch follows the forge remote's branch of the same name when that is
  exactly the head, so a pull or push goes where the author's does.
- Uncommitted changes carry across, or git refuses and says what would be
  overwritten; nothing is stashed or discarded.
- Afterwards: "On <branch>", with why when it is `pr-N`, or what it follows.
- The worktree code (`pull::open_worktree`, `worktree_path`, the
  `review_worktree` command and FEAT-089's two tests of them) goes: nothing
  else used it. FEAT-089's sweep rows for it no longer apply.
- The *Added* changelog entry for the worktree is replaced; it was unreleased.

## Acceptance criteria

- On a pull request whose branch is not here, Check out branch leaves the
  repository on that branch at the pull request's head.
- No local branch other than `pr-N` is ever moved.
- A conflicting uncommitted change stops it with git's message, and the
  working copy is as it was.
