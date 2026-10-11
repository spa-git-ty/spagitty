<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-117 — Every merge happens in Merger

**Status:** Backlog — reported 2026-10-11, not started.
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
- Pull's *Merge if it cannot fast-forward* in the toolbar stays where it is: it is part of pulling, not a merge someone starts. Awaiting the author's confirmation.
- Farm task merges also go through Merger. Awaiting the author's confirmation, since the Farm merges tasks as part of its own run.

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
