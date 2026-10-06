<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-100 — Plan

**Item:** [`agile/items/FEAT-100-a-merger-that-shows-the-result-first.md`](../items/FEAT-100-a-merger-that-shows-the-result-first.md)

## Approach

**One dry run per pair, A merged with B.** The conflicts of B into A and of A
into B are the same regions with the sides named the other way round, and the
screen draws A | Result | B whatever the direction. So the backend answers per
pair (`merger::forecast`), and everything that depends on the direction or the
strategy is a pure function of that answer in `src/lib/merger/plan.ts`.
Switching direction never asks git again.

**The dry run** is `git -c merge.conflictStyle=diff3 merge-tree --write-tree -z
--messages A B`, which prints the tree, every conflicted stage, and the
messages, and writes only objects nothing points at. diff3 is asked for so
each region carries its base, which the Base strip (FEAT-102) shows. A git
older than 2.38 (the linked dev machine has 2.34.1) has no `--write-tree`, so
`merger::preferred` reads the version once and falls back to a scratch
worktree: `worktree add --detach` under `<git dir>/spagitty/merger/`, `merge
--no-commit --no-ff`, `ls-files -u` and the marked-up files, then `worktree
remove --force` and `prune`, by a guard that does it on drop whatever
happened. Both paths produce the same `Outcome`, and both are tested.

**The forecast** adds, per side, the commits since the merge base, the newest
three, the tip's time, whether it is checked out (from `worktree list`), and
how many of its commits touch a conflicted file — the most times a rebase of it
can stop. Files are classified from `diff --name-status --no-renames base..A`
and `base..B` against the conflicted set.

**git calls** stay in `shell.rs` (BUG-046: the only place a process starts).
Several Merger commands exit 1 to say "no" — `merge-base`, `merge-tree`,
`merge-base --is-ancestor` — so `run_extra` takes the exit codes that are
answers, an environment (for the private index of FEAT-101) and stdin.

**The command** `merger_forecast` lets the session go before the merge is
worked out, as `review_conflicts` does, so a large merge does not hold the
repository for every other screen.

**The screen** is `src/routes/merge/+page.svelte` over
`src/lib/merger/MergerPlan.svelte`, with `BranchCard`, `BranchPicker` (the
shared `Menu`), `HistoryAfter` and `SideBadge`. The store keeps the pick per
repository in `localStorage` — a convenience — and asks again whenever
`repo.token` moves, because a plan for commits that are no longer the tips is
a plan for some other merge. A and B are `--side-a` (`--lane-5`) and
`--side-b` (amber, lighter in dark), tokens in `app.css`.

## Risks

- `merge-tree`'s `-z` layout: parsed field by field and stopped at the first
  field that is not a stage, so the messages section can change without
  breaking it. Covered by a parser test on a captured output.
- A remote branch or tag picked as the receiver: refused in the plan with the
  reason, not left to fail at write time.
