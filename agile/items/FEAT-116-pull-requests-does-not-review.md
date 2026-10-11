<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-116 — Pull requests lists and reads; Review reviews

**Status:** Done — on `feature/FEAT-116-pull-requests-does-not-review`; unmerged. Merging still happens here until FEAT-117.
**Screens:** 1H, 1R.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

Both screens can review a pull request, so it is not clear which one is for reviewing. Pull requests has:

- a *Review* button and *Reply in Review*, which open the review room (FEAT-105);
- draft comments and a *Publish Review* modal with *Approve* and *Request changes*;
- a verdict picker and a review comment box in `RequestDetail.svelte`.

The author's rule: Review is where pull requests are reviewed. Pull requests only shows them.

## Change

- Every review entry leaves the Pull requests screen:
  - the *Review* and *Reply in Review* buttons (`PRWorkspace.svelte`, around lines 71–88, 184–188 and 471);
  - the draft comments and the *Publish Review* modal (around lines 452–575);
  - the verdict options and review comment in `RequestDetail.svelte` (around lines 50–53, 217 and 242).
- Pull requests keeps listing, reading (description, files, threads), creating, editing and closing pull requests. Merging moves to Merger (FEAT-117).
- The store code only these used (`requests.reviewing`, the publish path) is removed rather than left unused.

## Acceptance criteria

- Pull requests has no control that comments on, approves or requests changes on a pull request.
- Review does everything that was removed: comments, replies, verdicts.
- Tests for the removed paths are removed, and a test asserts the screen offers none of them.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/requests/PRWorkspace.svelte`, `src/lib/requests/RequestDetail.svelte`, `src/lib/requests/store.svelte.ts`, `src/lib/requests/workspace.test.ts`.
- FEAT-105's item should say it was reversed here.
- **Branch:** `feature/FEAT-116-pull-requests-does-not-review`, with plan, testing documents and a changelog entry when the work starts.

## What was done

Pull requests lists and reads; Review reviews. Gone from the Pull requests screen: the *Review* and *Reply in Review* buttons, the draft inline comments and the *Publish Review* modal with its verdict choices, the *Reviewer View* / *Developer View* switch, the inline composer, reply box and *Mark as resolved* in the diff, and the verdict picker and review comment in the side panel (`RequestDetail`). What stays: the list, the description, the files, the commits and the existing comments (read, with their *Resolved* mark), a footer that counts unresolved and resolved comments, creating, editing the draft state and closing. Merge stays on the screen until FEAT-117 moves it. From the store, `review()`, `reviewing`, `reviewError`, the draft comments with their local storage, `replyToComment`, `resolveComment` and `role` are removed rather than left unused. Review already did all of it (FEAT-087 to FEAT-105), so nothing was added there. FEAT-105's *Review from the pull request screen* is reversed by this.
