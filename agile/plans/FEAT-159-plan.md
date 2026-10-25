<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-159 — Plan

**Item:** [FEAT-159](../items/FEAT-159-clear-all-notifications.md)

1. `src-tauri/src/inbox.rs`: `Inbox::clear(before, repository)` and `inbox_clear`'s optional `repository`.
2. `src/lib/notifications/center.svelte.ts`: `clear(beforeMs, repository)` and `clearAll(repository)`.
3. `src/lib/notifications/NotificationCenter.svelte`: the *Clear all* button and its confirmation, scoped like *Mark all read*.
