<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-159 — Automated test record

**Item:** [FEAT-159](../items/FEAT-159-clear-all-notifications.md)

- `src/lib/notifications/center.test.ts`: *Clear all* asks with the count and the scope, cancelling keeps everything, agreeing empties the drawer and disables the button; `clearAll` with a repository keeps the others.
- `src-tauri/src/inbox.rs`: `clearing_one_repository_leaves_the_others`; `clearing_forgets_only_what_is_older` with no repository.

## Results — 2026-10-25

- `bunx vitest run src/lib/notifications`: 32 passed.
- `cargo test -p spagitty --lib inbox`: 7 passed.
