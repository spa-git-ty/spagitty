<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-039 — Plan

**Item:** [`agile/items/BUG-039-a-graph-test-that-reads-the-clock.md`](../items/BUG-039-a-graph-test-that-reads-the-clock.md)

## Approach

`Fixture::git` gains a private `git_with` that adds environment variables, and
`commit_at` / `commit_all_at` pass `GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE`
as `@<seconds> +0000`. `woven` dates its five linear commits from `WOVEN`
(1,700,000,000): Initial import +0, Add notes +60, Rewrite line 3 +120,
Rewrite line 38 +180, Start the split view +240. The merge, the tags and the
stash stay on the wall clock, which is always later, so they are still the
newest things in the repository; so is anything a test commits on top.

Only `woven`. The other fixtures' tests are about content, conflicts and
states, not time.

The todo test becomes two assertions: the sorted ids equal
`git rev-list --no-merges v0.1.0..HEAD` sorted, and no id is listed before its
parent.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/fixture.rs` | `WOVEN`, `commit_at`, `commit_all_at`, `git_with`; `woven` dated. |
| `crates/spagitty-core/src/rebase.rs` | The todo test, and the doc comment it tested. |

## Risks and rollback

- A test elsewhere that assumed `woven`'s commits are recent. The whole
  workspace passes, on Windows and Linux. Rollback is a revert.
