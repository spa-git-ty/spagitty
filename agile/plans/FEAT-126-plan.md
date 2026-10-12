<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-126 — Plan

**Item:** [FEAT-126](../items/FEAT-126-merger-says-a-branch-is-behind-its-remote-and-pulls-it.md)

## Steps

1. **The core.** `ops::pull_branch(repo, name)`: look the branch up in the worktrees (`merger::checked_out`, which the forecast already uses). Checked out somewhere: `git pull --ff-only` there. Checked out nowhere: `shell::fetch_into`, which reads `branch.<name>.remote` and `branch.<name>.merge` and fetches that ref into the branch. A branch that tracks nothing is refused in words before git is asked.
2. **The command and its call.** `pull_branch` beside `pull`, on the session like it.
3. **What the cards know.** The store already reads the branch rows for the pickers; the rows carry each branch's upstream and how far it is from it. Keep the ones that are behind, by name.
4. **The words.** `behindLine` and `pullRefusal` in `plan.ts`, each with a test.
5. **The card.** A line and a button under the name, only when the branch is behind.
6. **Afterwards.** `repo.refresh()` moves the repository's token, the page primes Merger again, the tips have changed, and the forecast is asked for again (the question it answers includes both tips).

## What decides it is right

- Tests against a real bare remote: a branch checked out nowhere moves and the checked-out branch does not; the checked-out branch moves and its files follow; a branch with commits of its own is refused and left where it was; a branch that tracks nothing says so.
- The lanes are let go however the pull ends.
- No branch is ever moved anywhere but forward.
