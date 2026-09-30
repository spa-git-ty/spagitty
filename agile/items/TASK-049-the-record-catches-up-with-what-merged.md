<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-049 — The record catches up with what merged

**Status:** Done.
**Branch:** `task/TASK-049-the-record-catches-up-with-what-merged`
**Screens:** none. It is the working record.
**Raised by:** the review of 2026-09-30, which listed nine items as still open:
the farm's reliability fixes, three tasks from the same week, and two graph
features.

## Problem

The index said nine items were being worked on. None was:

| Item | Where it went |
| --- | --- |
| TASK-036 | Pull request #32, 2026-09-05. |
| TASK-032 | Pull request #33, 2026-09-05. |
| BUG-024 | Pull request #34, 2026-09-05. |
| BUG-025 | Pull request #35, 2026-09-05. |
| BUG-026 | Pull request #36, 2026-09-05. |
| BUG-027 | Pull request #37, 2026-09-05. |
| TASK-033 | Pull request #38, 2026-09-05. |
| FEAT-079 | Merged into `main` on 2026-09-07; released in 0.7.0. |
| FEAT-081 | Released in 0.8.0 and corrected in 0.8.1, 2026-09-13. |

Every one of the seven pull requests is in `main` and in every release since
0.5.1. Their branches have nothing `main` lacks. The items said `Open` because
nobody went back to them after the merge — the drift TASK-012 made the index
test for, in a form the test cannot see: a status that agrees between item and
index but not with the history.

## Change

- The seven merged through pull requests are `Fixed` or `Done`, with the pull
  request that merged them on the status line.
- Each claim was checked against today's tree, not only the history: the farm
  suites the bugs added pass (331 unit and 53 pipeline tests, in WSL), and the
  process-containment tests TASK-033 put on Windows pass natively
  (`execution::tree`, 2; `verification::command`, 16).
- FEAT-079 and FEAT-081 are merged but their sweeps were never run, and
  FEAT-081 was reopened precisely for being called done without one. The rows
  that can be driven in the release build were run on 2026-09-30 and recorded;
  what that settles is on each item's status line.

## Non-scope

- The farm's other tests on native Windows, which start `/bin/sh` as a
  stand-in agent. They are not in TASK-033's Windows job and never were.
- Changing any code.

## Acceptance criteria

- No item says `Open` for work that is merged, unless something it promised is
  still unchecked, and then its status line says what.
