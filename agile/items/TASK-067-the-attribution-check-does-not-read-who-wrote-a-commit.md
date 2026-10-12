<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-067 — The attribution check does not read who wrote a commit

**Status:** Backlog — raised by a review of the 2.0.1 stack, 2026-10-12.
**Screens:** —.
**Raised by:** the review: one commit in pull request #63, f475d8b, has *Antigravity* as its author, and gate 1 passed it.

## What is missing

TASK-064's check reads commit messages and the pull request's description. A commit whose author or committer is an agent carries no line for it to find.

## Why it waits

The check reads every commit of the pull request it runs on. Added now, it would fail #63 and anything stacked on it for f475d8b, and the author's decision of 2026-10-12 is that what is already written stays as it is (`AGENTS.md`, TASK-063). Once #63 is merged that commit is history, which the check does not read, and this can be added and pass.

## Acceptance criteria

- A pull request with a commit whose author or committer is an agent fails gate 1, naming the commit.
- The same test of name and address as a co-author's (BUG-078): a person who shares a name with an agent passes.
- Nothing before the pull request is read.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `tools/attribution.mjs`: `git log --format` can give `%an <%ae>` and `%cn <%ce>` beside `%B`; `isAgent` already takes that shape.
- **When:** after #63 is merged.
