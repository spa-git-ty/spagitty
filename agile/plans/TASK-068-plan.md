<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-068 — Plan

**Item:** [TASK-068](../items/TASK-068-what-coderabbit-found-on-63-and-64.md)

Twenty-four open threads on the two pull requests, some of them the same
finding twice. Each was read against the code as it stands, not as the comment
describes it, and sorted into what is true and worth a change, and what is not.

The two that matter most:

- **A Farm task and the way it is merged.** `mark_merged` accepts a task when
  its own commit is in its target. A squash or a rebase writes other commits,
  and another direction puts the result elsewhere, so the Farm would refuse
  for good and the task would stay in review with its work merged. Opened for a
  Farm task, Merger marks squash and rebase as not possible with the reason,
  and keeps the direction. The Farm's own merge was always a merge commit.
- **A push that ends before its token is known.** The worker is started before
  the command returns its token. A done event with a token the store does not
  have yet was dropped. While an operation is starting, such events are kept,
  and the one whose token comes back is read then.

The rest are one change each, listed in the item. Every thread gets an answer
and is resolved, changed or not.
