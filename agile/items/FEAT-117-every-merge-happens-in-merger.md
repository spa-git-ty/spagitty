<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-117 — Every merge happens in Merger

**Status:** Done — on `feature/FEAT-117-every-merge-happens-in-merger`; unmerged. Not tried against a real host or a real Farm run.
**Screens:** 1S, 1H, 1A, 1Q.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

Merges can be started in four places, and each does it its own way:

- **Pull requests:** the *Merge Pull Request* modal (`PRWorkspace.svelte`, around line 599) calls `requests.merge()`, which asks the host (`api.mergePullRequest`). `RequestDetail.svelte` line 244 has a disabled *Merge*.
- **Graph:** the commit menu's *Merge* and *Merge, always commit* (`src/lib/graph/actions.ts`, lines 305 and 311).
- **Farm:** a task is merged with `api.mergeTask` from `Building.svelte` and `TaskJourney.svelte`.
- **Merger** itself.

The author's rule: every merge is done in Merger.

## Change

- Each other place offers *Open in Merger* instead, with the pair and direction already chosen (`merger.preset`).
- Merger gets a pull request mode:
  - it merges through the host with the host's methods (merge, squash, rebase);
  - when the host refuses because of conflicts, it resolves them locally, pushes, and then merges.
  
  BUG-063's conflict marks lead here.
- Pull's *Merge if it cannot fast-forward* in the toolbar stays where it is: it is part of pulling, not a merge someone starts. Confirmed by the author, 2026-10-11.
- Farm task merges also go through Merger. Confirmed by the author, 2026-10-11.

## Acceptance criteria

- No screen but Merger makes a merge commit or asks a host to merge.
- *Open in Merger* from a pull request, a graph commit or a Farm task opens Merger with that pair and direction ready.
- A pull request can be merged from Merger with each method its host allows.
- A pull request with conflicts can be resolved and merged from Merger without leaving it.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Order:** after FEAT-116 (which empties the Pull requests screen of review), and with pull request #57 reworked to match (TASK-062).
- **Where:** `src/lib/merger/` (store, plan, the merge dialog), `src/lib/requests/`, `src/lib/graph/actions.ts`, `src/lib/farm/components/`, `src-tauri/src/forge_bridge.rs`.
- **Branch:** `feature/FEAT-117-every-merge-happens-in-merger`, with plan, testing documents and a changelog entry when the work starts.

## What was done

The author confirmed on 2026-10-11 that Farm task merges move to Merger too, and that Pull's *Merge if it cannot fast-forward* stays where it is.

- **Pull requests** (`PRWorkspace`, `RequestDetail`): the *Merge* button and the *Merge Pull Request* modal are replaced by **Open in Merger**, which opens Merger on the pull request. The store's `merge`, `merging` and `mergeError` are gone.
- **Graph:** the branch menu's *Merge*, *Merge, always commit*, *Fast-forward* and *Rebase* are one entry, *Merge into <current> in Merger*, which opens Merger with that branch coming into the checked-out one (dragging a label onto another already did). The frontend `integrate` and `api.integrate` are removed.
- **Farm:** *Merge it in Merger* on the task opens Merger on the task's branch and the branch it was meant for (`farm/merge.ts`); when the merge is written, Merger tells the Farm (`farm_task_merged`, `Service::mark_merged`), which checks the task's commit is in the destination before finishing the task and removing its worktree. The Farm's own merge command is no longer reachable from the interface. **What stays:** at the *Automatic* autonomy level the Farm still merges verified, reviewed tasks itself, in the backend, because the person chose that level to say *without asking*.
- **Merger** (`MergerOrigin.svelte`, `merger/store.svelte.ts`): a *pull request* mode that merges through the host with *Merge commit*, *Squash* or *Rebase* (the host's own sentence if it refuses), and, when the pull request conflicts, *Resolve the conflicts here*: it fetches and checks out the pull request's branch, plans the target coming into it like any two branches, and after the merge is written offers *Push <branch>* and *Back to the pull request*, which refreshes the pull request and offers the host merge. The three methods are offered for every host; a host that has not got one answers in its own words.
- `tools/merges.test.ts` fails when any frontend file but Merger's calls anything that writes a merge.
