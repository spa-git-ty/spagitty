<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-063 — Plan

**Item:** [TASK-063](../items/TASK-063-audit-the-merged-pull-requests.md)

Read each merged pull request's diff beside `AGENTS.md`: first the five small
ones and #61 whole, then #52 by what it can do wrong — what an agent may
write, what a remote agent may read, what the network client sends, how the
prompt frames what it reads — and by every CodeRabbit finding checked against
the code now, not against what the thread says. Each real defect becomes an
item; what works and differs from the house style goes in one task; one
verdict per pull request goes in the item.

Then the one change the audit asked for: a check in gate 1 that refuses agent,
tool and session attribution in a pull request's commits and description
(TASK-064). History on `main` is the author's to decide and is not touched.
