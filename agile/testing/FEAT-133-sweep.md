<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-133 — Sweep

**Item:** [FEAT-133](../items/FEAT-133-recipes-git-steps-in-blocks-run-with-an-agent.md)

The rows marked *headless* were run on 2026-10-13 against the built frontend in headless Chrome at 1440×900, dark and light, with a stand-in for the backend: a small validator and a pretend run. That shows the screen. It does not show git, the restore point or an agent. Those were run by the Rust tests on scratch repositories on Windows, and an agent was not run at all. None of these rows has been run in the running application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT133-01 | A repository open | Press **Recipes** on the rail | It is in Repository, after Review. The blocks are on the left in six groups, the stack in the middle, how it runs on the right | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT133-02 | A new recipe | Press `S`, `L`, `M`, `P`, `C`, `U` | Stash, Pull, Merge, Pop stash, Commit and Push are added in that order, each with its command beside it | P1 | One key passed headless. All of them are covered by mounted tests. |
| SWEEP-FEAT133-03 | A stack of several blocks | Press `j` and `k`; Alt with ↑ and ↓; Delete; Ctrl+Z | The selection walks, the block moves, the block goes, and it comes back | P1 | Delete passed headless. The rest covered by mounted tests. |
| SWEEP-FEAT133-04 | A Merge block | Press its branch | The branch menu opens with this repository's branches, searchable. The one chosen is in the block and in its command | P1 | Covered by a mounted test. Not run in the application. |
| SWEEP-FEAT133-05 | A Pop with no Stash above it | Look under the block and at **Run** | *There is no Stash above this to pop.* is under the block, and **Run** says *1 problem* and cannot be pressed | P1 | Passed headless with the stand-in validator. The rule is a Rust test. |
| SWEEP-FEAT133-06 | A branch that is pushed, with nothing ahead | Build Amend, then Push | The push says step 1 rewrites commits the remote already has, and the stack cannot run | P1 | Not run. Covered by Rust tests of the rule and of what is read from the repository. |
| SWEEP-FEAT133-07 | Changes in the working tree | Build a Merge with nothing above it | A warning that nothing stashes or commits them. The stack can still run | P2 | Not run. Covered by a Rust test. |
| SWEEP-FEAT133-08 | *Agent help* off | Look at the right-hand panel and at a Commit block | The sentence and the note are disabled, *agent writes* cannot be chosen, and the panel shows the commands as a script. The commit's message is typed | P1 | Passed headless. |
| SWEEP-FEAT133-09 | An agent set up in Settings › Agents | Switch *Agent help* on, write a sentence, press **Propose blocks** | The agent's blocks fill the stack with what it said about them. Nothing has run. Ctrl+Z brings back what was there | P1 | Passed headless with a made-up answer. **No agent was asked.** |
| SWEEP-FEAT133-10 | As above, with an agent that answers with a block that does not exist | Press **Propose blocks** | The answer is refused, and the stack is as it was | P1 | Not run. Covered by a Rust test. |
| SWEEP-FEAT133-11 | *Agent help* on | Read the brief | `recipes.md` with its version, then this recipe: its steps, the allowed commands, *Never*, and the note | P1 | Passed headless with the stand-in's brief. The real one is compared whole in a Rust test. |
| SWEEP-FEAT133-12 | A model reached over an API that this repository has not agreed to | Choose it as the agent | It asks before anything is sent, once. On no, it is not chosen | P1 | Not run. Covered by a store test. |
| SWEEP-FEAT133-13 | The spec's recipe at *Ask every step* | Press **Run**, then go on at each step | A restore point is saved first. It stops before every step with the command it is about to run | P1 | Passed headless with the pretend run. The stops are Rust tests on real repositories. |
| SWEEP-FEAT133-14 | The same at *Ask before pushing* | Press **Run** | It runs to the Push and stops before it | P1 | Not run. Covered by a Rust test. |
| SWEEP-FEAT133-15 | The same at *Read-only* | Look at **Run** | It cannot be pressed, and says the script is the way out. **Copy as script** copies the steps as a shell script | P2 | Covered by a mounted test. The script is compared whole in a Rust test. |
| SWEEP-FEAT133-16 | A branch that conflicts with `main` | Run a recipe that merges `main` | The run stops on the conflict with the file named, and offers *Open in Merger*, *Open in Conflicts*, to go on, to abort the merge, and to end | P1 | Passed headless. A Rust test on a real conflict. |
| SWEEP-FEAT133-17 | Stopped as above | Press **Open in Merger**, make the merge there, come back and go on | The merge is aborted in the working copy, Merger opens on the two branches, and the run goes on once the merge is made | P1 | Not run. The run's half is a Rust test; Merger's half was not driven. |
| SWEEP-FEAT133-18 | Stopped as above | Press **Open in Conflicts**, resolve, come back and go on | The merge is committed and the next step runs | P1 | Not run. Covered by a Rust test that resolves the file by hand. |
| SWEEP-FEAT133-19 | Stopped as above | Press **Abort the merge** | The merge is aborted, the run ends, and the steps before it are kept | P1 | Not run. Covered by a Rust test. |
| SWEEP-FEAT133-20 | A Commit block with nothing to commit | Run | The run stops with git's words, and offers to run it again, skip it or end | P2 | Not run. Covered by a Rust test. |
| SWEEP-FEAT133-21 | *Agent help* on, a Commit set to *agent writes* | Run | The agent writes one line from the changes, shown in its colour, and the commit has that message and no trailer | P1 | Not run. **Needs an agent.** The message's handling is a Rust test. |
| SWEEP-FEAT133-22 | As above, a step that fails | Run | The stop says what the agent said went wrong | P2 | Not run. **Needs an agent.** |
| SWEEP-FEAT133-23 | A run waiting before a step | Commit something in a terminal, then go on | The run says the repository changed while it waited, before it runs anything | P1 | Not run. Covered by a Rust test. |
| SWEEP-FEAT133-24 | A run that stashed untracked files, merged and committed | Press **Undo the whole run** | After asking, `HEAD`, the branches, the index, the working tree with its untracked files and the stash list are as they were | P1 | Not run from the screen. Covered by Rust tests on real repositories, through the command too. |
| SWEEP-FEAT133-25 | A run that pushed | Press **Undo the whole run** | Everything local goes back. The question said that what was pushed stays pushed | P1 | Not run. The wording is covered by a store test. |
| SWEEP-FEAT133-26 | A recipe has been run | Open **Reflog** | *Restore points* lists it with its recipe, where `HEAD` was and how long ago, with *undo to here* and *drop* | P1 | Passed headless. |
| SWEEP-FEAT133-27 | A step is running | Look at the toolbar and press **Fetch** | The toolbar names the run and its step, and Fetch waits with the reason | P2 | Not run. The lanes are covered by a store test. |
| SWEEP-FEAT133-28 | A run is waiting | Close Spagitty and open it again | The builder is shown. The restore point is in Reflog | P2 | Not run. |
| SWEEP-FEAT133-29 | A saved recipe | Press **Export**, then **Import** in another repository and paste | The recipe opens there, not yet saved, under a name nothing has | P2 | Covered by mounted tests. |
| SWEEP-FEAT133-30 | Light theme | Open Recipes, and a run | The blocks, the squares and the stop card read as clearly as in dark | P2 | Passed headless. |
| SWEEP-FEAT133-31 | Windows, macOS and Linux | Run the spec's recipe on each | The same steps, the same result | P1 | Not run. The Rust tests were run on Windows only. |
