<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-155 — Untrack: stop tracking files on a branch, in a commit, and push it

**Status:** Done — on `fix/BUG-097-sign-in-and-gitlab-list`; unmerged. Covered by Rust and component tests; not looked at in the running application (the author is checking it).
**Branch:** `fix/BUG-097-sign-in-and-gitlab-list`.
**Screens:** 2B (Untrack).
**Raised by:** the author, 2026-10-20: "Untrack , i want a screen you design following our design persona that allow me to untrack file or folder on specific branch and push a commit with it".

## Change

- **A screen, 2B Untrack**, in Tools after the Reflog. On the left a local branch, the checked-out one at first, and the files and folders tracked on it, read from git at that branch, with *Find a file*. Ticking a folder takes in everything inside it.
- **On the right, everything that will happen before any of it does:** the paths; a `.gitignore` line for each (`/path`, `/folder/`, glob characters escaped), on by default; the commit's message, suggested from the paths until it is typed over; *Push after committing*, on by default. One button: *Untrack, commit and push*.
- **On the branch checked out here** the commit is made in place, hooks and signing as configured. It is refused while anything else is staged, because it would go into this commit, and while `.gitignore` has uncommitted changes that adding to it would commit.
- **On another branch** the commit is made in a worktree with nothing checked out — its index is the branch's tree, its folder empty — so the working copy and the checked-out branch do not move. Signing applies; hooks do not run, since the branch's files are not on disk for them, and the screen says the commit was made without them. The worktree is removed afterwards, whether or not the commit was made.
- **The push** is the network worker's, as every push is, with `--set-upstream`: a branch that is not on the remote yet is created there. It goes to the branch's upstream remote, else `origin`, else the only remote. It signs in with the connected account (BUG-097).

## Acceptance criteria

- Untracking on the checked-out branch removes the paths from the commit, leaves them on disk, and adds their `.gitignore` lines; `git status` is then clean.
- Untracking on another branch moves only that branch: the checked-out branch, the index and the working copy are as they were, and no worktree is left behind.
- Staged changes, or an uncommitted `.gitignore` when lines would be added, refuse the commit with a sentence saying what to do.
- The push starts after the commit when asked, and not when there is no remote.
