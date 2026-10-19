<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-144 — Automated test record

**Item:** [FEAT-144](../items/FEAT-144-the-landing-lane.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| The lane | `crates/spagitty-farm/tests/lane.rs` | A task lands on `main` while the person is on another branch: their branch and files are untouched, the result is a merge commit, and one `land` entry names the task, its agent, `before:skipped` and the new tip. A destination that moved is merged again and checked before it moves (`before:pass`, the old tip recorded), and a checked-out destination is brought forward with `--ff-only`. A conflict blocks the task with *Conflicts with main*, moves nothing, writes no conflict into the working copy and leaves no ledger entry. A candidate that fails its checks goes back to its agent with *Merged with main, the checks fail*, nothing moves, and the scratch worktree is gone. A task merged in Merger gets a `merger` entry. |
| The ledger | `crates/spagitty-farm/src/persistence/store.rs` | Appended in sequence; a torn last line is dropped; stored as `ref` and `kind: "land"`. |
| Existing merges | `crates/spagitty-farm/tests/pipeline.rs` | A merge puts the work in the person's branch and removes the worktree; a merge is never a fast-forward; a branch switch does not redirect a task's merge, which now lands on the recorded branch without the person going back to it. |
| The screen | `src/lib/farm/describe.test.ts`, `src/routes/farm/page.test.ts` | The ledger in sentences — who made it, who passed it, how it was checked; a Merger merge; a lost race — and Activity's *Branch moves*, newest first, with old → new. |

## Not covered

- Not looked at in the running application; the author is checking it.
- The lane's suite uses `/bin/sh` agents: it runs in WSL (Arch), not natively on Windows. `Fixture`-based Merger tests for `candidate` and `advance` are covered through the lane, not on their own.
- A destination moved by a second thread between the read and the move (the race) is covered by the compare-and-swap in `update-ref` and Merger's existing stale tests, not by a timed test here.
- Landing several tasks at once, after-landing checks and reverts are not in this item.

## Results — 2026-10-18

- `cargo test -p spagitty-farm` (WSL, Arch): 444 unit, 5 lane, 55 pipeline, 14 supplemental and 7 wizard passed, 0 failed.
- `bunx vitest run src/lib/farm src/routes/farm`: 149 passed, 0 failed.
- `bun run check`: 0 errors, 0 warnings.
