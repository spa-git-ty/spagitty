<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-101 — Automated test record

**Item:** [`agile/items/FEAT-101-merge-without-conflicts.md`](../items/FEAT-101-merge-without-conflicts.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/merger/land.rs` | A merge into the checked-out branch moves it and its files, with both parents and the default message; **a merge into a branch that is not checked out leaves the checked-out branch, an uncommitted change and the index byte for byte as they were**; a squash is one commit with the target as its only parent and the typed message; a new branch starts from A and nothing else moves; a rebase replays the source on top of the target, a straight line, the source unmoved and no worktree left; a fast-forward only moves the pointer, and is refused when both sides have their own; a branch that moved since the plan is refused as stale; a remote branch cannot receive; a conflicted merge needs every path resolved, then lands the text; the scratch-worktree fallback builds the same tree as `merge-tree`; text with markers left in is refused and nothing moves; taking the side that deleted a file deletes it; uncommitted work in the way is refused by git and nothing moves. |
| `src/routes/merge/page.test.ts` | Merge now opens the dialog with the default message and writes nothing until its button; the typed message, the tips and the target are what is sent; the done state and Merge another; Back writes nothing; a refusal is shown in the dialog; a rebase has no message box and sends none; a new branch sends its name and the message names it. |

## Test command and output

On Windows 11, git 2.55:

```
$ bun run check
COMPLETED 1247 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Test Files  161 passed (161)
     Tests  3417 passed (3417)
$ cargo test -p spagitty-core
test result: ok. 627 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
```
