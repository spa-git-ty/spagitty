<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-067 — Plan

**Item:** [TASK-067](../items/TASK-067-the-attribution-check-does-not-read-who-wrote-a-commit.md)

1. In `tools/attribution.mjs`, read the range a second time as `%H%x00%an <%ae>%x00%cn <%ce>`.
2. `findAgentWriters` passes each author and committer through the same `isAgent` a co-author trailer goes through, so BUG-078's people still pass, and names the commit by its short hash and which of the two it was.
3. `commits` mode reports both kinds of finding together; gate 1 already runs it on `base..head` only, so nothing before the pull request is read.
4. Unit-test an agent author, an agent committer, and people who share a name with an agent; run the CLI on this branch and on a range of history that holds f475d8b.
