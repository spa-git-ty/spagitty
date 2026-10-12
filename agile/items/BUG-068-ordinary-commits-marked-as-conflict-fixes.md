<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-068 — Ordinary commits are marked as conflict fixes

**Status:** Open — on `bugfix/BUG-068-ordinary-commits-marked-as-conflict-fixes`; two of the three suspects are confirmed and fixed, the pull request the author saw it on has not been checked.
**Screens:** 1R.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## What happens

In the review room's file list, some files carry *conflict fix* and their changes get the *Conflict fix, not in the author's own commits* card. They hold an ordinary commit from the pull request's author, and no conflict was resolved.

A wrong mark is worse than none: it tells the reviewer to read the author's own work as somebody else's.

## Suspected cause

Not reproduced yet. The detection is `crates/spagitty-core/src/remerge.rs`, `conflicts()`, from FEAT-092. Three suspects:

1. **Regions with no conflict count.** `ConflictFix` documents that `main_side` and `branch_side` are both empty "where the merge changed lines that nothing conflicted over". Those regions are still reported as fixes, so any line a merge commit changed outside a conflict is marked.
2. **Merges of any branch count.** `merges()` takes every two-parent commit between the merge base and the head. A `git pull` merge of the pull request's own branch, or a merge of another feature branch, is treated like a merge of the target.
3. **Line endings.** On Windows with `core.autocrlf`, the remerge can differ from the committed merge by line endings alone, which would mark whole runs of lines.

## Acceptance criteria

- A line is marked as a conflict fix only when it was written to resolve a region that had conflict markers, in a merge whose other parent is reachable from the target.
- A pull request whose author merged the target cleanly and then committed normally shows no conflict fixes.
- Each suspect above that turns out to be real has a fixture test. The first fixture is made from the pull request the author saw this on.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Reproduce first:** ask the author which pull request and file showed it, and run `git show --remerge-diff <merge>` on it to see which suspect it is.
- **Where:** `crates/spagitty-core/src/remerge.rs` (`regions`, `merges`, `conflicts`), `src/lib/review/rows.ts` (how fixed lines become cards), `src/lib/review/RoomFiles.svelte`.
- **Branch:** `bugfix/BUG-068-ordinary-commits-marked-as-conflict-fixes`, with plan, testing documents and a changelog entry when the work starts.

## What was done, and what was not

Two of the suspected causes were real, each shown by a fixture test that fails without the fix:

- **Regions with no conflict (suspect 1).** `regions()` reported lines a merge changed away from any conflict as a place "with no sides", and `conflicts()` turned them into fixes. A region now carries `conflicted`, true only for lines written inside or straight after a pair of conflict markers, and only those become fixes.
- **Merges of any branch (suspect 2).** `conflicts()` took the second parent as the target's when neither parent was in the target. A merge now counts only when exactly one parent is reachable from the target; the pull request's own branch pulled back, or another feature branch merged in, is skipped and not listed among the merges re-done.

A clean merge of the target followed by an ordinary commit marks nothing (a control that passed before and after).

Suspect 3 (line endings under `core.autocrlf`) is not proven either way: a line-ending difference outside a conflict is now dropped with the first fix, but one inside a conflicted region would still be shown. The author's own pull request (`git show --remerge-diff <merge>`) is the way to settle it.
