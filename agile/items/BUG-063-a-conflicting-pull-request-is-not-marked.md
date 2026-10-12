<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-063 — A pull request that conflicts with its base is not marked, and Merge only fails

**Status:** Fixed — on `bugfix/BUG-063-a-conflicting-pull-request-is-not-marked` (pull request #57 rebuilt on the stack that makes every merge Merger's, FEAT-117); unmerged.
**Screens:** 1R, 1H, 1S.
**Raised by:** the author, 2026-10-08, while preparing a demo of reviewing a conflicting pull request (`maxmya/trattoria-demo#3`, which GitHub reports as `mergeable: CONFLICTING`).

## What happens

- **Review (1R):** the inbox card, the preview card and the review room say nothing about the conflict. The room's *Conflict fixes 0* filter is about resolutions made inside the pull request, so it reads as if all is well. A reviewer can approve a pull request that cannot be merged as it stands.
- **Pull requests (1H):** the list and the workspace say nothing either, and *Merge* is enabled. *Confirm Merge* sends the merge, GitHub refuses, and the dialog shows `github.com: Pull Request has merge conflicts`. There is no next step.
- **Merger (1S)** can resolve exactly this: A = the base, B = the pull request's branch, landing *Into* the branch, then push. Nothing on 1R or 1H leads there.

## Acceptance criteria

- A pull request the host reports as conflicting is marked as such on its inbox card, in the review room's header, and in the Pull requests list and workspace.
- *Merge* on such a pull request explains why it cannot merge before sending anything, and offers *Resolve in Merger* with the pair and the direction (*Into* the pull request's branch) already chosen.
- After the branch is resolved and pushed, the mark clears on the next refresh.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Data:** GitHub's pull request has `mergeable` / `mergeable_state` (REST) or `mergeable` / `mergeStateStatus` (GraphQL). GitLab has `merge_status` / `detailed_merge_status`. Check whether the forge layer (`src-tauri/src/forge_bridge.rs`, `crates/spagitty-core`) already reads it before adding a field to `PullRequest` in `src/lib/types.ts`. The value can be `UNKNOWN` until the host has computed it, so plan for a later refresh.
- **Merger hand-off:** `merger` has a `preset` used by `prime()` (`src/lib/merger/store.svelte.ts`). Setting `{ a: base, b: head, into: 'b' }` before navigating to `/merge` should open the right plan.
- **Reproduce:** `maxmya/trattoria-demo#3` (`feature/new-heading-color` into `main`) is a disposable conflicting pull request.
- **Branch:** `bugfix/BUG-063-a-conflicting-pull-request-is-not-marked`, with plan, testing documents and a changelog entry when the work starts.

## Fix

The forge layer already reads the host's answer into `PullRequest.mergeable`; only `false` counts as conflicting, because `null` means the host has not decided. The mark is now shown in **Review** (a red *conflicts with main* tag on the inbox card, *Conflicts with main: it cannot be merged as it stands* first in the preview, and *Conflicts with main* in the room's header line) and in **Pull requests** (a *conflicts* chip on the list row, in the side panel and in the workspace header). The mark comes from each re-read of the list, so it clears on the first refresh after the branch is resolved and pushed.

What pull request #57 did in the Pull requests screen's merge dialog (a message and *Resolve in Merger* in place of *Confirm Merge*) is not here: with FEAT-117 that screen no longer merges, and *Open in Merger* shows the conflict and offers *Resolve the conflicts here* — fetching and checking out the pull request's branch, planning the target coming into it, pushing it, then merging through the host. That is the "Merger needs the branch to exist locally" limitation of #57, removed. #57's failing *code quality* check was a type error: it added `Chips.base` and not to what `chipsOf` returns on the *Back with you* path, which `main` gained in BUG-064; both returns carry it now.
