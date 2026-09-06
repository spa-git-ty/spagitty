<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-036 — Plan

**Item:** [Use the current shared amendments book](../items/TASK-036-sync-amendments.md).

## Approach

Follow Amendment 11: keep `docs/AMENDMENTS.md` as a pointer to the shared book.
Copying the whole book would introduce a second source that could drift.

1. Read the canonical book and compare the repository pointer against it.
2. Correct the frozen range, dates, and amendment subjects; label obsolete
   claims as historical rather than silently presenting them as current rules.
3. Add a binding-amendments section to the existing `AGENTS.md`, retaining its
   other contents, and record the change in the index and changelog.
4. Run the existing tests and build, inspect the diff, and record results.

## Files

`docs/AMENDMENTS.md`, `AGENTS.md`, `CHANGELOG.md`, `agile/README.md`, and this
item's documents. No runtime code or dependencies change.

## Risks and rollback

The canonical path is local to the author's machine. Agents on another machine
must report that it is unavailable rather than inventing its contents.
Rollback is a revert of this item's changes; the shared book is never changed.
