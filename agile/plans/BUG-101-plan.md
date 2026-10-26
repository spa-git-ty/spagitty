<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-101 — Plan

**Item:** [BUG-101](../items/BUG-101-every-edit-in-calls-says-could-not-save.md)

1. `src/lib/calls/store.svelte.ts`: `writeCollection` passes the stem as `was`, so the store writes over the collection's own file.
2. Leave the store's guard in `calls/store.rs` as it is: it is what stops a new collection taking an existing file.
