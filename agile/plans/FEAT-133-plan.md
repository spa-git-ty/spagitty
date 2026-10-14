<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-133 — Plan

**Item:** [FEAT-133](../items/FEAT-133-recipes-git-steps-in-blocks-run-with-an-agent.md)

Written on 2026-10-13, before the code, after the author's note of that day and their three answers to it.

## What the author's answers change

The item was written with an agent in every recipe. It is now a switch on the recipe, *Agent help*:

- **On.** The person writes a sentence, the agent proposes the blocks, and the person reviews and edits them before anything runs. During a run the agent writes a message set to *agent writes*, and explains a failure. The note and the brief are there.
- **Off.** The sentence, the note and *agent writes* are disabled. No agent is called. The run is the blocks' git commands and nothing else. A commit message and a new branch's name are still typed.
- **Either way** Spagitty runs each command itself, a step at a time. *Copy as script* gives the steps as a bash script.

## The pieces

1. **The model** in `crates/spagitty-core/src/recipes/`. A recipe is a name, the branch it runs on, its steps, a level, the switch, an agent and a note, in the JSON form of the spec (`"v": 1`). A step is one of the eighteen blocks with its parameters. The catalogue is closed: the blocks are an enum, and there is no variant for a force push, `-D` or `reset --hard`.
2. **The commands** (`command.rs`): each block to the git arguments it runs, as a list and never as a line for a shell. The same function gives what is shown beside the block and what is run.
3. **The validator** (`validate.rs`): one pure function of the recipe and a `World`, which is what is true of the repository: its branches, remotes and checks, what is checked out, whether the tree is dirty, how many commits the upstream lacks, which branches may not be pushed to unattended, and whether the agent chosen can be used. It walks the steps keeping what they would leave: the branch checked out, the branches made, the stashes pushed, whether pushed commits were rewritten. The spec's errors and warnings are its rules, each with a code. It runs on every change and again at *Run*.
4. **The brief** (`brief.rs`): `recipes.md`, compiled in with its version, followed by the generated section in the spec's form. With the switch off there is no brief, because nobody is briefed.
5. **The proposal** (`propose.rs`): what the agent is asked when the person writes a sentence, and how its answer is read. It is given the catalogue as examples and the repository's branches, remotes and checks, and must answer with steps in the JSON form. The answer is parsed into the same enum, so a block that is not in the catalogue is not a step, and is then validated like any other stack. Nothing it proposes runs: it lands in the builder.
6. **The script** (`script.rs`): the steps as a POSIX shell script with `set -e`, each step a commented line.
7. **The restore point** (`restore.rs`), written before the first step under `refs/spagitty/restore/<id>/`: the commit `HEAD` was at and the branch's name, the tip of every branch the recipe names, each stash, the index as a tree, and the working tree as a tree that includes untracked files, made through an index of its own so the real one is not touched. *Undo the whole run* ends an operation left in progress, puts the branches and `HEAD` back, then the working tree and the index, then the stash list. The newest 20 are kept and none older than 7 days.
8. **The run** (`run.rs`): a state machine that is stepped. `tick` runs one step or stops; `answer` takes the person's choice. What stops it:
   - the level: every step at *Ask every step*, Push and Delete branch at *Ask before pushing*, nothing at *Full access*; *Read-only* runs nothing at all;
   - *Stop and ask me*, at any level;
   - a conflict, a non-zero exit, a failed check, a rejected push, a tree that is not clean at *Stop unless clean*, and the repository having changed since the last step, at any level.
   What it needs from outside is a trait: saying a line, running a check, asking the agent for a message, asking it to explain a failure. Tests give it a made one.
9. **The commands** in `src-tauri/src/recipes.rs`: recipes kept per repository in application data; one call that validates and returns the findings, each step's command, the brief and the script; the proposal; a run started on a thread of its own, answered, stopped and undone; and the restore points. A run's record is written beside the recipes after every change.
10. **The screen** at `/recipes`, code 1V, a rail row in Repository after Review:
    - the catalogue by group, each block with its key;
    - the stack: *On Run*, each block's parameters as small fields, its command beside it, *Move up* and *Remove*, and the validator's findings under the block they are about;
    - the panel: *Agent help*, the agent, the level, the sentence and *Propose blocks*, the note, the brief, and *Run · N steps*;
    - the run view: the steps with state and time, the commands and their output, the stop as the one tinted card with its choices, and the restore point with *Undo the whole run*.
11. **Reflog** lists the restore points, each with *Undo to here* and *Drop*.

## A conflict

A Merge block's conflict offers *Open in Merger*: the merge in the working copy is aborted and Merger is opened on the two branches, where an agent can be assigned as in 2.0. When the merge has landed, *Continue* checks the branch is merged and goes on. Every other conflict (a pull, a rebase, a cherry-pick, a popped stash) offers *Open in Conflicts*, and *Continue* finishes the operation once nothing is conflicted. *Abort* ends the operation and the run, keeping the steps before it.

## Dependencies

None.

## What decides it is right

- Every validator rule has a test with a stack that breaks it and one that does not.
- A recipe run on a scratch repository leaves what the same commands typed by hand leave.
- *Undo the whole run* after a stash with untracked files, a merge, a commit and a new branch leaves the repository as it was: `HEAD`, the branches, the index, the working tree with its untracked files, and the stash list.
- With the switch off no agent is called at any point, and a stack with *agent writes* in it cannot run.
- No stack, proposed or imported, can run a command the catalogue does not have.
