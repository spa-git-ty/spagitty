<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-132 — Investigate: the life of a file, a folder, a branch or a line

**Status:** Backlog — recorded 2026-10-12 for 3.0.0; not started.
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
