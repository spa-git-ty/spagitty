<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-025 — One completion watcher per run

**Status:** Open
**Branch:** `bugfix/BUG-025-single-run-watcher`
**Screens:** Farm (1Q).

## Problem and reproduction

watch_all selects Running tasks even after their sessions are claimed. An empty waiter still invokes watch_all again, producing redundant threads. Review runs remain in Review and are missed. Completion currently finds the newest run by task rather than the run actually waited for.

## Scope and acceptance criteria

Every implementation and review run has one completion owner. An empty claim does not reschedule watchers. Completion applies only to its own run, errors are visible, and cancelled attempts cannot alter a retry.

## Non-scope

New product features, unrelated formatting, and destructive cleanup of authored work.
