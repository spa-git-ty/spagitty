<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-103 — Automated test record

**Item:** [`agile/items/FEAT-103-a-rebase-that-stops-in-merger.md`](../items/FEAT-103-a-rebase-that-stops-in-merger.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/merger/replay.rs` | A replay stops on the commit that conflicts — step 1 of 2, the commit named, A as main and B as feat, diff3 markers, the replayed commit on B's side — and neither branch, the index nor the checked-out branch moves; coming back finds the same stop and the same worktree; resolving then finishing moves the target to a straight line of both commits, keeps each message and the resolved text, never moves the source, and leaves no worktree; into B the columns still read A then B, and taking A takes main's lines; skipping drops the stopped commit; aborting leaves every ref, the worktree list and `git status` as they were, and nothing can be finished after; a replay with nothing in its way is done at once; markers swap sides with their base. |
| `src/routes/merge/rebase.test.ts` | Resolve starts the rebase and shows *Rebasing … onto … · commit 2 of 5* and the commit, Continue held until the stop is resolved, Skip this commit; Continue sends the stop's resolutions, then the dialog, Finish the rebase through `merger_rebase_finish` and never `merger_land`; Skip; Abort asks and returns to the plan; with nothing in the way Merge now goes straight to the dialog. |
| `src/routes/merge/page.test.ts` | A rebase with no conflicts replays first and finishes through `merger_rebase_finish`, with no message. |

## Test command and output

On Windows 11, git 2.55:

```
$ bun run check
COMPLETED 1253 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Test Files  163 passed (163)
     Tests  3391 passed (3391)
$ cargo test -p spagitty-core
test result: ok. 645 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
$ cargo check -p spagitty
(clean)
```
