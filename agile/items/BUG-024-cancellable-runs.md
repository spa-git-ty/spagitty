<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-024 — Stop a run while its waiter owns it

**Status:** Open
**Branch:** `bugfix/BUG-024-cancellable-runs`
**Screens:** Farm (1Q).

## Problem and reproduction

Start a slow agent, begin await_task on another thread, then cancel. The existing waiter removes the session from the only map used by cancellation, so the agent keeps writing after the task reads Cancelled. The same ownership gap affects a collected planner.

## Scope and acceptance criteria

A watched task and a collected planner stop promptly, descendants cannot keep writing, terminal state is preserved, and service locks are never held while joining pipes. Existing farm data stays readable. Verification command cancellation is completed with the verification runner item.

## Non-scope

New product features, unrelated formatting, and destructive cleanup of authored work.
