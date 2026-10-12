<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-123 — Merger checks out the branch it merged into, and offers to push it

**Status:** Done — on `feature/FEAT-121-say-what-is-running`; unmerged. Seen in the built frontend against fixtures; not yet in the running app.
**Screens:** 1S.
**Raised by:** the author, 2026-10-12, on 2.1.0: after choosing the branch a merge lands on, that branch should be checked out, with *push now* or *cancel* offered.

## Problem

Merger lands a merge without checking anything out (FEAT-101). When the receiving branch was not the one checked out, the merge was made and the working copy was still on another branch. The next two steps were always the same and both were elsewhere: switch to the branch, then push it from the toolbar.

## Change

When a merge chosen in Merger has landed:

1. **The receiving branch is checked out.** The toolbar says *Switching to main…* while git does it (FEAT-121).
2. **The done dialog says** *main is checked out*, and offers **Push now** and **Not now**, with *Open in Graph* beside them.
3. **Push now** pushes that branch to its upstream and waits for git to say how it went, as a pull request's branch is pushed (BUG-076). The button's title says where: *Push main to origin/main*. When it is done the dialog says *Pushed to origin/main*.
4. **Not now** closes the dialog. The merge stays made and unpushed.

Landing itself is unchanged: the merge is still made from the dry run, with nothing checked out. The checkout comes after, and is never the merge's failure.

### What is not a plain checkout

| Case | What happens |
| --- | --- |
| The receiving branch is already checked out | Nothing is checked out and nothing is said about it. Landing already brought the files forward. |
| It is checked out in another worktree | It stays there, and the dialog says where. git allows a branch to be checked out once. |
| git refuses, usually for uncommitted work in the way | The dialog says *Merged, but main could not be checked out:* and git's sentence, with **Check out main** to ask again. The push is still offered. |
| The merge went into a new branch | The new branch is checked out. |

### Where the push goes

- To the branch's upstream, when it has one on a remote of this repository. Both ends are named when the upstream has another name (`main:trunk`).
- Otherwise to the same name on `origin`, or on the first remote when none is called that. `git push` is given the remote, so the branch is set to track it.
- With no remote, no push is offered and the dialog says *No remote to push to*.

The remote and the branch are always named, so the push does not depend on the checkout having worked.

### The words

*Not now*, not *Cancel*. The merge is made when the dialog opens, and a button called *Cancel* would promise to take it back.

## Acceptance criteria

- After a merge into a branch that is not checked out, that branch is checked out and the dialog says so.
- **Push now** sends that branch to its upstream and says *Pushed* only when git has.
- A refused checkout leaves the merge made, shows git's words and can be asked for again.
- A rejected push shows git's words and can be pressed again.
- A pull request resolved in Merger and a Farm task are left as they were.

## What was done

- `merger.settle`, `checkOutLanded` and `pushLanded` in `src/lib/merger/store.svelte.ts`; `pushPlan` in `plan.ts`.
- The done state of `MergeDialog.svelte`.
- Where the push goes is read beside the checkout, so the dialog's buttons are the same from the moment it opens.

## Not in this change

- **A pull request resolved here** already has its branch checked out and is pushed to the pull request's own branch (BUG-076). Unchanged.
- **A Farm task** is merged where the Farm meant it to go, and the working copy is not moved for it (BUG-077).
- A merge an agent lands at *Unattended* is treated as any other merge chosen here: the receiving branch is checked out.
