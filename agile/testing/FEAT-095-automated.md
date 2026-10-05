<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-095 — Automated test record

**Item:** [`agile/items/FEAT-095-check-out-a-pull-request.md`](../items/FEAT-095-check-out-a-pull-request.md)

## What was tested

In `crates/spagitty-core/src/pull.rs`, against a host repository with a pull
request and a clone that fetched it:

- `a_pull_request_is_checked_out_under_its_own_name`: a new `feature` at the
  head, checked out; again from `main`, the same branch.
- `the_remote_s_own_branch_is_followed`: with `origin/feature` at the head,
  the new branch's upstream is `origin/feature`.
- `a_branch_of_yours_is_never_moved`: a local `feature` elsewhere stays put
  and `pr-7` is checked out; a source named like the target takes `pr-7` and
  `main` stays put.
- `its_own_pr_branch_moves_forward_and_never_back`: after the author pushes,
  `pr-7` moves to the new head; with a commit of yours on it, it is refused and
  not moved.
- `uncommitted_work_it_would_overwrite_stops_it`: a conflicting edit stops it;
  still on `main`, the edit intact.

In `src/routes/review/page.test.ts`: the button fetches then checks out with
the pull request's names, says "On <branch>" and what it follows, refreshes
the repository; says why when it is `pr-N`; says what git said when it fails.
No *Open in worktree* is left.

## Test command and output

On Windows 11:

- `cargo test -p spagitty-core --lib` — 590 passed.
- `cargo clippy -p spagitty-core --all-targets -- -D warnings` — clean.
- `cargo check -p spagitty --all-targets` — builds; its three warnings are in
  `desktop.rs`, which this change does not touch.
- `bun run test` — 3234 passed, 1 failed: `tools/record.test.ts` on BUG-043's
  missing documents, on `main` and not this change. `bun run check` — 0 errors,
  0 warnings.

## What is not covered automatically

A real pull request in the release build. See the sweep.
