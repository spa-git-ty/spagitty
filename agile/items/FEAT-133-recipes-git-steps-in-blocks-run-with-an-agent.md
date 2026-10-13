<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-133 — Recipes: git steps in blocks, run with an agent

**Status:** Done — on `release/3.0.0`, after FEAT-130; unmerged. The author answered the proposal's question 1 on 2026-10-13, and added a note the same day that makes the agent's help a switch. Covered by Rust tests on real repositories, of every validator rule, the restore point and the run, through the commands on a mock application too, and by mounted tests of the screen. Looked at in the built frontend in headless Chrome with a stand-in for the backend. Not looked at in the running application, and no agent was asked anything.
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

## The author's note of 2026-10-13

Sent while FEAT-130 was being finished, in these words: "recpies should be created include help from the agent ( if toggeled on ) if not , plain text input is disabled , the run is pure git commands with bash".

Three questions were asked about it the same day, and the author chose:

- **A switch for the agent's help**, on the recipe.
- **On, the agent builds the blocks.** The person writes what they want in a sentence, the agent proposes the stack from the catalogue, and the person reviews and edits it before anything runs. During the run the agent also writes a message set to *agent writes*, proposes a conflict's resolution in Merger, and explains a failure.
- **Off, only the agent's text is disabled**: the sentence to the agent, the note, and *agent writes*. A commit message and a new branch's name can still be typed, because git needs them.
- **Off, no agent is called**, and the run is the blocks' git commands and nothing else.
- **Either way Spagitty runs each command itself, a step at a time**, so the levels, the stops, the command log and *Undo the whole run* all work. *Copy as script* gives the same steps as a bash script.

This goes further than the acceptance criteria above, which have an agent in every recipe. Where the two disagree, this section is the later word.

## Not in this item

- **Blocks outside the catalogue**, loops, and conditions beyond the three guards.
- **Recipes across repositories**, scheduled recipes, and recipes committed to the repository (the spec's question 6).

## What was built

- **`crates/spagitty-core/src/recipes/`**:
  - `mod.rs`: the model. A step is an enum of the eighteen blocks, so what is not in the catalogue cannot be written down.
  - `command.rs`: a block to its git arguments, the same list for what is shown and what is run.
  - `validate.rs`: one pure function of the recipe and what is true of the repository, with a code for each rule.
  - `world.rs`: what git knows of the repository, read in-process.
  - `brief.rs` and `recipes.md`: the guide, compiled in with its version, and the generated section.
  - `propose.rs`: what the agent is asked for a stack, and how its answer is read.
  - `script.rs`: the steps as a POSIX shell script.
  - `restore.rs`: the restore point under `refs/spagitty/restore/<id>/`, and undoing a whole run.
  - `run.rs`: the run, stepped. `tick` runs one step or stops, and `answer` takes the person's choice.
- **`shell::ran`** in the core: a git command whose exit is an answer and not an error, recorded in the command log like every other.
- **`src-tauri/src/recipes.rs`**: fifteen commands. Recipes and each run's record are kept per repository in the data directory. A run is on a thread of its own and sends the whole run when it changes and each line as it is printed.
- **`agents::ask`**: one question to an agent and its answer as text. A command-line agent is run in an empty directory of its own, in its read-only mode.
- **The screen** is `src/routes/recipes/+page.svelte` with `src/lib/recipes/`: the catalogue, the block cards, the agent's panel, the run view, the store and the keys. A rail row in Repository after Review.
- **Reflog** lists the restore points, with *undo to here* and *drop*.

## Differences from the spec and the artboard

- **The agent's help is a switch**, and off by default. The spec has an agent in every recipe. This is the author's note of 2026-10-13.
- **The agent proposes the blocks** from a sentence of at most 280 characters. The spec has no such thing.
- **The levels have the names 2.0 shows them under** since TASK-060: *Read-only*, *Ask every step*, *Ask before pushing*, *Full access*. The spec and the artboard have *Suggest*, *Step*, *Sign off* and *Unattended*, which are still the stored values.
- ***Copy as script* writes a POSIX shell script on every platform.** The spec has PowerShell on Windows. The author's answer was a bash script.
- **A conflict has no *Let the agent propose* button.** A Merge block's conflict is taken to Merger, where an agent is assigned as in 2.0. Every other conflict is taken to Conflicts.
- **A run does not stop itself when the repository changes under it.** It says so when the person next answers a stop, which is the only time a run waits. The spec has the watcher stop it.
- **Pop has no parameter.** It pops the newest stash a Stash above it made.
- **Reword is the last commit's message.** The spec's parameter is a commit.
- **Cherry-pick's commits are typed**, and checked to be single words that are not options.
- **Run checks picks from the farm's checks**, which is where 2.0's merge checks are configured.
- **The restore point keeps the working tree as a tree made through an index of its own**, not as a `git stash create` commit, which leaves untracked files out.
- **The run view has no *only allowed commands · 0 blocked* line.** Spagitty runs the commands, so there is nothing to block.
- **The JSON form has `agentHelp`.**
