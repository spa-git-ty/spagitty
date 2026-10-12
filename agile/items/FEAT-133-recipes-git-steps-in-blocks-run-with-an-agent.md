<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-133 — Recipes: git steps in blocks, run with an agent

**Status:** Backlog — recorded 2026-10-12 for 3.0.0; waits on the author's answer to the proposal's question 1.
**Branch:** `feature/FEAT-133-recipes-git-steps-in-blocks-run-with-an-agent`.
**Screens:** 1V (Recipes, new, at `/recipes`), chrome (a rail row), 1S (Merger receives a recipe's conflict), 1M (Reflog lists restore points).
**Raised by:** the author, 2026-10-12: an area to talk to the selected agent and tell it, with building blocks, what to do with the git history — stash > pull > merge > pop > commit > push — with limited text entry, ready-made blocks covering every structure, and a markdown file Spagitty ships to guide the agent to exactly what the person wanted. The same day: the look must be for professionals and enthusiasts, not a toy.
**Proposal:** [`docs/proposals/workbench-and-recipes.md`](../../docs/proposals/workbench-and-recipes.md) §8 and Appendix A. **Designs:** [`design_handoff_workbench/`](../../design_handoff_workbench/) — `Recipes.dc.html` (interactive), `RecipeRun.dc.html`.

## Change

A recipe is a stack of blocks from a closed catalogue. Each block has:

- parameters chosen from menus;
- its git command shown beside it;
- a one-key shortcut.

Spagitty validates the stack, writes the agent's brief, saves a restore point, and runs the recipe at one of the 2.0 levels. A conflict goes to Merger. *Undo the whole run* puts everything back.

## Acceptance criteria

- **The rail**: a row in Repository, after Review, with code 1V.
- **The catalogue** is the spec's table (§8.2): 18 blocks in six groups. There is no force push, no `-D` and no `reset --hard` anywhere. Branches and remotes are picked from the branch menu (FEAT-122) and the remotes list.
- **Free text** is limited to a message (72 characters) and the note (140), and is validated by length and `check-ref-format` where it names a branch.
- **The validator** is one Rust function used on every change and again at *Run*, with the spec's errors and warnings (§8.3). *Run* is disabled while any error stands, and says how many.
- **The brief** is `recipes.md` (compiled in, versioned, its version shown) followed by the generated section in the spec's form (§8.5). The person sees exactly what will be sent.
- **Execution**:
  - Execution follows the author's answer to question 1.
  - With the proposed answer, Spagitty runs every command through the process runner and the activity lanes (FEAT-121), logs each in the command log (FEAT-020), and calls the agent only for an *agent writes* message, a conflict (a Merger assignment, FEAT-115/FEAT-117), and explaining a failure.
  - The agent never writes in the working copy.
- **Levels**: Suggest, Step by step, Sign off and Unattended behave as in the spec (§8.4).
- **Always stops**: a conflict, a non-zero exit, a failed check, a rejected push, or something changing the repository mid-run always stops, at any level.
- **The restore point**:
  - It is written before the first step, under `refs/spagitty/restore/<id>/`: HEAD, every named branch's tip, the stash list, and the working tree and index as a `stash create` commit.
  - *Undo the whole run* restores them.
  - The newest 20 are kept, none older than 7 days, and they are listed in Reflog.
- **The run view** shows the steps with state and time, the agent's card with commands and output, the stop as the one tinted card with its choices, and the restore point.
- **The record** of each run is kept in application data and never sent anywhere.
- **Storage**: recipes are kept per repository in application data. *Copy as script* writes POSIX shell, or PowerShell on Windows, and recipes export and import as JSON (`"v": 1`).
- **Look**: neutral block cards with a small muted square per group, and no bright fills (the author's direction).
- **Tests**: Rust tests for every validator rule, brief generation, restore and undo on a scratch repository, and each level's stops. Component tests for the builder's keys and live validation.

## Not in this item

- **Blocks outside the catalogue**, loops, and conditions beyond the three guards.
- **Recipes across repositories**, scheduled recipes, and recipes committed to the repository (the spec's question 6).
