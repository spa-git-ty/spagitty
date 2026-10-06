<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-103 — A rebase that stops, in Merger

**Status:** Backlog
**Screens:** 1S.
**Raised by:** the author, 2026-10-06. Slice 4 of `design_handoff_merger/`.

## Problem

A rebase applies one commit at a time and can stop on each. Merger has to resolve each stop with the same columns and offer Continue, Skip and Abort.

## Change

- Rebase then fast-forward runs in a scratch worktree, never in the user's.
- Each stop is resolved with the shared resolver; Continue, Skip, Abort.
- When it finishes, the receiving branch fast-forwards to the result.

## Acceptance criteria

- The source branch is never moved; Abort leaves the repository as it was.
