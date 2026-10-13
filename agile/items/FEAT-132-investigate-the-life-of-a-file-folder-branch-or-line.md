<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-132 — Investigate: the life of a file, a folder, a branch or a line

**Status:** Done — on `release/3.0.0`, after FEAT-131; unmerged. Covered by Rust tests against real repositories and by mounted tests of the page, and looked at in the built frontend in headless Chrome with a stand-in for the backend. The Rust and the screen have not been run together: not looked at in the running application.
**Branch:** `feature/FEAT-132-investigate-the-life-of-a-file-folder-branch-or-line`.
**Screens:** 1O (Investigate, at `/investigate`, replacing File history at `/history`), chrome (a rail row).
**Raised by:** the author, 2026-10-12: pick a file, a folder or a branch and see its history, with blame: when it was created and how it was updated through time.
**Proposal:** [`docs/proposals/workbench-and-recipes.md`](../../docs/proposals/workbench-and-recipes.md) §7. **Designs:** [`design_handoff_workbench/`](../../design_handoff_workbench/) — `Investigator.dc.html`.

## Change

FEAT-063's File history becomes Investigate and goes on the rail. It tells a life:

- born when, by whom;
- how the target grew;
- when it was renamed, moved or rewritten;
- who wrote it over time, and who owns it today.

It covers four targets: file, folder, branch and line.

## Acceptance criteria

- **The rail and routes**: a rail row in Tools, after Rebase, with code 1O. `/history` redirects to `/investigate` with the same path, and `OFF_RAIL` loses its entry.
- **The four targets** are as the spec's table (§7.1) says:
  - **File** follows renames when *Follow renames* is on.
  - **Line** uses `git log -L`.
  - **Branch** is born at its merge base with the default branch, and reads the reflog when the branch is local.
- **Rewrite** is one commit that changed at least half the file's lines as they were then. The share is the named constant `REWRITE_SHARE`.
- **The life card** has the facts line, the size chart with born, renamed, moved and rewrite markers, and one row per author with a tick per commit. Agents are in `--agent`.
- **The events list** runs newest first, with kind, author, summary and `+/−`.
- **The selected event** shows:
  - the share of the file changed;
  - how many of its lines today's blame still gives it;
  - today's ownership by author;
  - its largest hunk;
  - *Open file at <sha>* (Files at that revision), *Compare with today* (Diff), *Show in Graph*.
- **Cost**: history is streamed, and the first paint comes from the first page. Results are cached by target and HEAD. At most 5,000 commits are read, then *Load more*.
- **Tests**: Rust tests for rename and move detection, the rewrite threshold, survival counting, and branch birth with a reflog and without one. Component tests for each target.

## Not in this item

- **Following code that moved between files** (`blame -C -C`).
- **Following into submodules.**

## What was built

- **`crates/spagitty-core/src/investigate.rs`**: `life` reads a page of a target's entries; `today` reads one blame for who owns a file and what survives of each commit; `largest_hunk`, `compare`, `branch_facts` and `default_branch`. `REWRITE_SHARE` and `LIFE_MAX` are named constants.
- **A file and a folder** come from `git log --raw --numstat -M`, a file with `--follow`. A file's size after each commit is counted from the object.
- **A line** comes from `git log -L`.
- **A branch's fork is not its merge base.** A branch that has had the default branch merged into it, or has landed, shares a late commit with it. The fork is found from the branch's own commits: the ones on its first-parent line that the other branch lacks, the oldest of which was made on the commit it forked from. Once a branch has landed, they are counted from the first parent of the commit that took it in.
- **Five commands**, `investigate_*`, none of which holds the session lock while it works.
- **The screen** is `src/routes/investigate/+page.svelte` with `src/lib/investigate/`: the store, `chart.ts` (the geometry, tested as arithmetic), the life card, the event list (a virtual list) and the selected event.
- **`/history` redirects** to `/investigate` with its path. `OFF_RAIL` has lost its entry. File history's store, view and command are removed.

## Differences from the spec and the artboard

- ***Compare with today*** shows the comparison in place, where the event's largest hunk is. The spec sends it to Diff, and Diff reads one commit, not two versions of a file.
- ***N / M of its lines still alive*** is the lines today's blame gives the commit, over the lines the commit added. The artboard's second number is the file's lines today.
- **The share a rewrite took** is the lines the commit removed over the lines the file had before it.
- **A folder's size** is summed from added and removed lines, from the oldest entry up. It is exact only once the whole life has been read.
- **A folder's event list** is the commits that added, removed or renamed a file, its birth, and its twenty biggest edits. The chart and the facts count every commit.
- **A branch's reflog** gives the finish of a rebase and a reset. The moves of each commit a rebase replayed are on `HEAD`'s reflog, not the branch's.
- **The first target** is the branch that is checked out, when the address names none.
- **A target is typed** into a field that searches the paths at the revision. The artboard draws a button that opens a dialog.
- ***Follow renames*** is a chip, like the switches in Files, and is offered for a file only.
- **A marker** is also drawn where a file was deleted and where a branch landed.
- **The range** is all time, the last year, the last 90 days or the last 30 days.
