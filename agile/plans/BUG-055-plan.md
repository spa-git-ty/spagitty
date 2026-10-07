<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-055 — Plan

**Item:** [`agile/items/BUG-055-branch-where-you-point.md`](../items/BUG-055-branch-where-you-point.md)

`Toolbar`: drop Clone and Rebase; Branch starts the field at `graph.selected` on `/`, else HEAD; Stash calls `actions.quickStash` (`dialog.prompt`, `api.stashPush(message, true)`). `CommitRows.createNamed` sets `creating` before `dialog.confirm` and refocuses on cancel. `review` store: `connected`, and `loadInvolved` checks accounts for the host before asking; `ReviewInbox` and the requests page centre `.empty`. Core `ignore.rs` (`Rules`, `any_not_ignored` via gix excludes, leading directories first); `watch.rs` watches the working tree, collects non-`.git` paths per burst, asks the rules at flush, and classifies refs by component.
