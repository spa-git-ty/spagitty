<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-104 — Plan

**Item:** [`agile/items/FEAT-104-a-branch-named-where-head-is.md`](../items/FEAT-104-a-branch-named-where-head-is.md)

`src/lib/graph/branching.svelte.ts` holds the commit the field is on and `branchName` (trim, spaces to dashes). `Toolbar` Branch becomes an action: `branching.start(head.id)` and `goto('/')`. `CommitRows` draws the field in that row's refs cell, reveals the row once, focuses on mount, and on Enter calls `actions.createBranchNamed` (replacing the prompt-based `createBranchAt`, whose only caller was the commit menu, which now opens the field).
