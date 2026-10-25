<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-159 — Clear all notifications

**Status:** Done — on `feature/FEAT-158-type-to-search`, in release 3.10.0; unmerged. Covered by store, drawer and Rust inbox tests; not looked at in the running application (the author is checking it).
**Branch:** `feature/FEAT-158-type-to-search`.
**Screens:** the notification drawer (the title bar's bell).
**Raised by:** the author, 2026-10-25: "also no clear all notifications option available we want to add it".

## Change

- *Clear all* beside *Mark all read* in the drawer's header. It deletes every notification in the scope the drawer shows: every repository's, or this repository's when *This repo* is picked. Read or unread, it asks first and says how many, since nothing brings them back. It is disabled when there is nothing to clear.
- `inbox_clear` takes an optional repository, as `inbox_read_all` already did, and `center.clearAll(repository)` uses it. Settings' *older than 90 days* clear is unchanged.

## Acceptance criteria

- Clear all, agreed to, empties the drawer and the stored inbox; cancelled, it changes nothing.
- With *This repo* picked, other repositories' notifications stay.
- **Tests**: the drawer's confirmation and result, the store's repository scope, and the Rust inbox's repository scope.
