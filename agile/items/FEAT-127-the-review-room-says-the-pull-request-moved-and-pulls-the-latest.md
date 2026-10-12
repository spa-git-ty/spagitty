<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-127 — The review room says the pull request moved, and pulls the latest

**Status:** Done — on `feature/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top`, with FEAT-124, FEAT-125, FEAT-126 and TASK-070; unmerged. Covered by mounted tests; not looked at in a browser or in the running app.
**Screens:** 1R.
**Raised by:** the author, 2026-10-12, on 2.2.0: "in review if the branch is not up to date, make it possible to pull latest".

## Problem

The review room reads a pull request once, when it is opened: it fetches the head and reads the files from disk. If the author pushes while it is open, nothing says so. The reviewer goes on reading, and commenting on, a version that is no longer the pull request. The only way to the new commits was to go back to the list and open it again.

## Change

- While a pull request is open in the room, its host is asked whether its head is still the one that was read: on arrival, when the window comes back to the front, and every three minutes.
- When it is not, the room says so across the top: *New commits on GitHub since this was read. What is shown is the pull request as it was.*
- **Pull latest** reads the pull request again from its new head.

## Decisions

- **What "not up to date" was taken to mean.** The pull request has commits on its host that the room has not read. The other reading, that the pull request's branch is behind the branch it targets, is a different thing with a different remedy (merging the target in, which writes to the author's branch) and was not built. It is under *Not done*.
- **Nothing is read until it is asked for.** The page is not changed under the reader. The room only says the pull request moved; the new commits are fetched on **Pull latest**.
- **What survives it.** The room is opened again as it is when a pull request is opened: ticks on files that changed are dropped and the room lands on the first file not viewed, so the reader is taken to what is new; pending comments are kept in the record, and those written against the old head are marked as they already were (FEAT-093).
- **A comment half written is not lost.** While the box is open **Pull latest** is disabled and says to save or cancel it first.
- **Asking costs one request for the list of pull requests**, the call the inbox already makes, so no new call was added to either host's client. It is not made more often than every thirty seconds, however often the window gains focus.
- **A host that cannot be reached says nothing.** Being offline is not news in the middle of a review.
- **An agent at work is told as before.** The room's head changing is what tells a running agent the pull request moved (2.0), and that still happens when the latest is pulled.

## Acceptance criteria

- With the room open, a push to the pull request is said within three minutes, or at once when the window is brought back.
- **Pull latest** shows the new head's files, with the count of viewed files true for it.
- Nothing is said when the host's head is the one being read.

## What was done

- `src/lib/review/room.svelte.ts`: `newer`, `check`. `src/lib/review/store.svelte.ts`: `update`.
- `src/lib/review/ReviewRoom.svelte`: the asking, the notice and the button.

## Not done

- **A pull request that is behind its target branch.** The room already says when it *conflicts* with it (BUG-063); it does not say when it is merely behind, and offers nothing for it.
- How many commits are new, and who pushed them.
- The inbox is not asked any more often than before; its *changed since* chip is as old as its last refresh.
