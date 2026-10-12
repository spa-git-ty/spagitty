<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-126 — Merger says a branch is behind its remote, and pulls it

**Status:** Done — on `feature/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top`, with FEAT-124, FEAT-125, FEAT-127 and TASK-070; unmerged. Covered by core tests against a real remote and by mounted tests; not looked at in a browser or in the running app.
**Screens:** 1S.
**Raised by:** the author, 2026-10-12, on 2.2.0.

## Problem

Merger plans a merge of two branches as they are on this machine. When one of them tracks a branch on a remote that has moved on, the plan is of a merge nobody wants: the conflicts, the files and the count of commits are all those of an old tip. Nothing on the screen said so, and catching up meant leaving Merger, checking the branch out and pulling.

## Change

- A branch card says when the branch it tracks has commits the branch lacks: *3 new commits on origin/main*.
- Beside it is **Pull now**, which brings the branch up to date whichever branch is checked out.
- The plan is worked out again against the branch as it is afterwards.

## Decisions

- **Only forward.** *Pull now* moves a branch to where its upstream is and does nothing else: no merge commit, no rebase. A branch that also has commits of its own cannot be moved that way, so its button is disabled and says why, and what to do instead: pick the remote branch as the other side, since merging it in is what this screen is for.
- **How a branch that is not checked out is moved.** `git fetch <remote> <ref>:refs/heads/<name>`. A fetch into a branch refuses anything that is not a fast-forward and touches no working copy, so it is safe with another branch checked out and uncommitted work on disk.
- **How a branch that is checked out is moved.** Git refuses to fetch into it, rightly: its files would no longer match it. It is pulled in the worktree it is checked out in, fast-forward only, so the files move with it. That covers this window's branch and one checked out in another worktree alike. When uncommitted changes are in the way git refuses and its sentence is shown; nothing is stashed on the person's behalf here.
- **The card knows what the last fetch knew.** How far behind a branch is comes from the remote-tracking refs on disk. Merger does not ask the remote on its own: it would be a network call nobody pressed a button for, on a screen that is opened often. **Fetch** in the toolbar brings the news, and the cards follow it. Recorded under *Not done* as the one thing a reader may expect and not find.
- **The lanes.** A pull holds `repository` and `network` (FEAT-121), under the name *Pulling main*, so the toolbar says it and nothing that would collide starts. While something else holds a lane the button waits and its title says for what.

## Acceptance criteria

- A card whose branch is behind its upstream says how far, and offers *Pull now*.
- *Pull now* works for the branch checked out here, for one checked out in another worktree, and for one checked out nowhere, without changing which branch is checked out.
- A branch that cannot simply move forward is not moved, and the card says why.
- A branch that is level, or tracks nothing, shows nothing new.

## What was done

- `crates/spagitty-core/src/shell.rs`: `fetch_into`. `crates/spagitty-core/src/ops.rs`: `pull_branch`, which chooses between it and a fast-forward pull in the branch's worktree.
- `src-tauri/src/commands.rs`: the `pull_branch` command. `src/lib/api.ts`: `pullBranch`.
- `src/lib/merger/plan.ts`: `Drift`, `driftsFrom`, `behindLine`, `pullRefusal`.
- `src/lib/merger/store.svelte.ts`: `driftOf`, `pulling`, `pullNow`. `src/lib/merger/BranchCard.svelte` draws it.

## Not done

- **Merger does not fetch on its own.** A card says a branch is behind only after something has fetched. If the cards should ask the remote when Merger opens, that is a setting and an item of its own.
- A remote branch or a tag picked as a side has no upstream and shows nothing.
- The commits that would come in are counted, not listed.
