<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-100 — Automated test record

**Item:** [`agile/items/FEAT-100-a-merger-that-shows-the-result-first.md`](../items/FEAT-100-a-merger-that-shows-the-result-first.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/merger.rs` | A git version read as major and minor; `merge-tree -z` read as a tree and its stages, and a clean one as none; the conflict and its base found by `merge-tree` and, the same, by the scratch-worktree fallback; the fallback leaves no worktree and nothing under `spagitty/merger`; the forecast's merge base, commits on each side, newest first, kind, checked-out worktree and commits touching a conflict; every file grouped as conflict, both, B's (added) and A's; a branch already merged; a name that is not a commit named in the error; a file deleted on one side conflicting as a whole; **the index file, every ref, HEAD and every working file byte-for-byte unchanged, and the index not rewritten, by either dry run and by a forecast**. |
| `src/lib/merger/plan.test.ts` | Roles for into A, into B and a new branch; the default new name; only a branch here receives; a new branch needs a free name; fast-forward refused with its reason, offered when the receiver is behind, and nothing at all when nothing comes in; the descriptions name the branches; the sentence per strategy; the three numbers; the conflict summary, a rebase's stops, and none; the meter per file and capped; resolve first or merge now; the card roles and arrows; every file's chip in both directions; the files line. |
| `src/routes/merge/page.test.ts` | The checked-out branch and the newest other as the pair, the dry-run chip, Lands here, the sentence and arrows; turning the direction round and Swap without asking git again; a new branch's name prefilled and followed; every strategy, fast-forward's reason, History after; picking another branch asks again and a clean pair offers Merge now; a remote branch cannot receive; the error. |
| `src/lib/nav.test.ts`, `src/lib/chrome/chrome.test.ts` | Merger on the rail after Conflicts and before Branches, on a quiet day too. |
| `src/lib/ui/flat.test.ts` | The new components read only defined tokens and no pixel type sizes. |

## Test command and output

On Windows 11, git 2.55:

```
$ bun run check
COMPLETED 1246 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS
$ bun run test
Test Files  161 passed (161)
     Tests  3410 passed (3410)
$ cargo test -p spagitty-core
test result: ok. 613 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
$ cargo check -p spagitty
(clean, after `bun tools/extensions/bundle.ts --debug` staged the bundle TASK-055 needs)
```

The scratch-worktree fallback is exercised on this git by calling the dry run
with `DryRun::Worktree` directly; it is the path a git older than 2.38 takes.
