<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-127 — Plan

**Item:** [FEAT-127](../items/FEAT-127-the-review-room-says-the-pull-request-moved-and-pulls-the-latest.md)

## Steps

1. **Ask.** `room.check()` asks for the open repository's pull requests, finds this one by number, and compares its head with the one read (`head.head`, the commit that was fetched, not the one the list had when the room was opened). A short id and a whole one for the same commit are the same. What comes back for a room that has since been left or reloaded is dropped, by the sequence number the room already keeps.
2. **Keep the answer.** `room.newer`: the pull request as the host has it, or null. Cleared when the room loads and when it is left.
3. **When.** An effect in the room: once it is ready, on the window's `focus`, and on a timer. `check` refuses to ask more than once in thirty seconds unless told to.
4. **Pull latest.** `review.update(pr)` replaces the open pull request with the host's. The room's own effect loads whatever is open, so it loads the new head by the path that opening a pull request takes.

## What decides it is right

- Nothing is fetched by the asking.
- The notice goes once the new head is read.
- A failure to ask leaves the room exactly as it was.
