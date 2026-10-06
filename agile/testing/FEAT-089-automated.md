<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-089 — Automated test record

**Item:** [`agile/items/FEAT-089-a-pull-request-read-from-disk.md`](../items/FEAT-089-a-pull-request-read-from-disk.md)

Written after the branch was built, on the tip of the stack that contains it
(`feature/FEAT-093-threads-done-properly`); the suites below are this item's,
and the counts are the whole run at that tip.

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/pull.rs` | Each host publishes its heads under its own ref; the head lands where no branch list reads and the target where a fetch puts it; a name that would change the refspec is refused; the worktree sits beside the repository; against a real host and clone: a pull request fetched and diffed from where it left its target, the change on the target since not counted; a whole file is every new line with the removed ones in place, with both blobs; the pull request opens in a worktree of its own and moves with its head. |
| `src/routes/review/page.test.ts` | The pull request put in a worktree of its own, fetched first, with a toast naming it; what git said when the worktree cannot be made. |

## Test command and output

On Windows 11, at the stack's tip:

```
$ bun run check
0 ERRORS 0 WARNINGS
$ bun run test
Tests  1 failed | 3153 passed (3154)
$ cargo test -p spagitty-core
test result: ok. 586 passed; 0 failed
```

The one failure is `tools/record.test.ts` on BUG-043, which reached `main`
without its plan and testing documents before these branches were cut.

In WSL (Arch), on this item's own branch when it was built and again at the
stack's tip:

```
$ cargo test -p spagitty --lib
test result: ok. 106 passed; 0 failed
```

## What is not covered automatically

Fetching from GitHub and from the author's work GitLab over the network, and a
remote that asks for credentials: the sweep.
