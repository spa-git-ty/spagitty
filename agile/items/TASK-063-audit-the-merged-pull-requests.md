<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-063 — Audit the pull requests merged since 2.0.0

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** —.
**Raised by:** the author, 2026-10-11: check the pull requests the author and Codex merged, for code style and correctness.

## Scope

| PR | What it is |
| --- | --- |
| #52 | Spagitty 2.0.0: agents in Review and Merger (FEAT-115), about 17,500 lines. Most of BUG-070 and TASK-060, and FEAT-118 and FEAT-119, start here. |
| #53 | BUG-059 — the Windows watcher and refs |
| #54 | BUG-060 — the Merger forecast on first open |
| #55 | BUG-061 — the Merger plan for the right pair |
| #56 | BUG-062 — back to the list after a merge |
| #58 | BUG-064 — a Review group for pull requests you reviewed |
| #61 | Restore release checks and prepare 2.0.1 (TASK-058) |

## Already found

**Attribution trailers on `main`.** Since 2026-10-01:

- 127 commit-message lines on `main` are `Co-Authored-By:` or `Claude-Session:` trailers;
- 29 commits are authored as *Claude*.

`AGENTS.md` forbids both. Removing them means rewriting history on a protected branch, which the author decides. The proposal is to leave history alone and stop new ones.

## Change

- Review each pull request on its own against `AGENTS.md` and the code around it: style, naming, comment density, tests that would fail without the change, records in `agile/`, changelog entries.
- Every real defect becomes a bug or task item. Style that differs from the house style but works is listed in one task, not fixed in passing.
- A check rejects agent, tool or session attribution in commit messages and pull request descriptions, so it cannot come back.

## Acceptance criteria

- Each pull request above has a written verdict, one paragraph, kept in this item.
- Each finding has an item, or is listed here as not worth one, with the reason.
- The attribution check runs in CI, and the author has decided what happens to the existing history.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- It is read-only until findings are filed, so it can run beside other work.
- Start with #52: it is the largest, and the bugs reported on 2026-10-11 point at it.
