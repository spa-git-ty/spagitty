<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-147 — Own copies, and intake at the door

**Status:** Backlog.
**Screens:** 1Q, 1K.
**Proposal:** [The landing lane and the local hub](../../docs/proposals/landing-lane-and-local-hub.md), §6 (named FEAT-141 there before the numbers were handed out).

## Change

An agent can work in a clone of its own (`git clone --local`, no remote) instead
of a linked worktree, so it cannot move the person's branches or push anywhere.
Spagitty fetches the work into `refs/spagitty/inbox/…`, checks it at the door
(ancestry, declared paths, protected paths, size, shape), and hands it to the
landing lane.

## Questions for the author

The proposal's §12: the hub's shape, the default for new farms, and hooks in
copies.
