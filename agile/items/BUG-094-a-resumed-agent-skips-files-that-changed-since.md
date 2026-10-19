<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-094 — A resumed agent skips files that changed since its first run

**Status:** Fixed — on `release/3.4.0`; unmerged. Covered by unit tests; not tried with a real agent in the running application.
**Screens:** 1R (Review), 1S (Merger)
**Raised by:** an unresolved CodeRabbit thread on pull request #52, carried by [TASK-066](TASK-066-style-notes-from-the-2-0-audit.md); confirmed in the code on 2026-10-19.

## What happens

*Resume* on a stopped review fetches the pull request as it is now and sends its current head. `assignment_start` then carries the earlier run on with `resumed`, which keeps the earlier record whole, its target included. When the pull request was pushed to in between:

- the agent reads the new head, and is told not to look again at the files it finished on the old one, though they may have changed;
- the record still names the old head, so what it proposes is kept against a commit that is no longer the pull request's.

A Merger assignment does the same when either branch moved.

## Acceptance criteria

- A run is carried on only when its job is at the same revisions: a review's base and head, a merge's two tips and base.
- Otherwise the start is a fresh run, and the agent is told that the earlier one moved and this one starts over.
- A unit test covers a review pushed to and a merge whose branch moved.

## Fix

`same_revisions` in `src-tauri/src/agents.rs` compares the two targets, and `assignment_start` keeps the earlier record only when it holds. A fresh run asked to resume one that moved has a sentence added to its note saying so.
