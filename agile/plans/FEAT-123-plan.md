<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-123 — Plan

**Item:** [FEAT-123](../items/FEAT-123-merger-checks-out-what-it-merged-into-and-offers-the-push.md)

## The reading taken

"When I choose the branch to land on, it should be checked out, and you may pop up push now or cancel." Read as: once the merge has landed, check out the branch that received it, and offer the push. Not: check out the branch the moment *Into main* is pressed, which would move the working copy for a plan that is still a dry run.

## Steps

1. **Landing does not change.** `merger_land` still writes from the dry run and checks nothing out (FEAT-101). No Rust is changed.
2. **`settle` runs after it**, only for a merge with no origin. It is not awaited by `land`: the dialog says the merge is made at once.
3. **The checkout** is `api.checkout` on the `repository` lane (FEAT-121). Three cases need none or cannot have one: already the branch here, checked out in another worktree (the forecast says where), and a refusal, which is kept as git's sentence with a way to ask again.
4. **Where a push goes** is `pushPlan(branch, rows, remotes)` in `plan.ts`: a plain function, so the cases are a table of tests. Read beside the checkout, so the buttons do not change when the checkout ends.
5. **The push** goes through the network store and waits on `whenDone`, as BUG-076 made a pull request's push do. The remote and the refspec are always given.
6. **The dialog's done state:** *Push now*, *Not now*, *Open in Graph*; after the push, *Open in Graph* and *Merge another*.

## What was decided on the way

- *Not now* and not *Cancel*.
- Where the push goes is the button's title and not another sentence.
- Nothing is said when the branch was already checked out.
- A pull request and a Farm task keep what they had.
