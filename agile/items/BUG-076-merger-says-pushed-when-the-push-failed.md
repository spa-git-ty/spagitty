<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-076 — Merger says *Pushed* when the push failed, and no fetch or push works after it

**Status:** Open — fixed on `bugfix/BUG-076-merger-says-pushed-when-the-push-failed` with its tests; Open until the sweep runs against a real host (2026-10-12).
**Screens:** 1S.
**Raised by:** a review of the 2.0.1 stack on 2026-10-12, before FEAT-117 was merged.

## What happens

A pull request that conflicts is resolved in Merger (FEAT-117): its branch is fetched and checked out, its target is merged in, and the result is pushed back with *Push <branch>*. That push was wrong in three ways:

1. It named no remote. The backend only passes a refspec to git after a remote, so git was run as a plain `git push` and the pull request's branch was never named.
2. It said *Pushed* as soon as git had started. A push that the host then rejected was shown as done, and its error went nowhere.
3. It did not go through the network store, which is what lets go of the worker when a fetch or a push ends. After one push from Merger, every fetch and push in the application was refused with *a fetch or push is already running* until Spagitty was restarted.

The suite passed because the test replaced the push with one that resolved at once.

## Acceptance criteria

- The push names the remote the pull request is on and the refspec `<branch>:<the pull request's branch>`.
- *Pushed* is shown only when git has said the push went through. A rejected push shows git's own words and leaves the button to press again.
- A fetch or a push started after a push from Merger is not refused.
- When Spagitty cannot tell that the pull request's branch is on a remote of this repository (a fork's is not), it does not push, and says so.
- Each of these has a test.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/merger/store.svelte.ts` (`resolvePullRequestLocally`, `pushPullRequestHead`), `src/lib/network/store.svelte.ts` (`whenDone`).

## Fix

`pushPullRequestHead` pushes through the network store with the remote named, and waits on the store's new `whenDone` for how the push ended. The remote is the one the target was fetched from. It is kept only when that remote is seen to have the pull request's branch, either because the checkout set it as the branch's upstream or because its remote-tracking branch is here; otherwise the push is refused with the reason.

## Left

A pull request from a fork cannot be pushed to from Merger: Spagitty does not know the fork's remote. The person is told to push it themselves. Reading the head repository from the host and pushing there would be a feature of its own.
