<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-036 — Plan

**Item:** [`agile/items/BUG-036-the-commit-list-a-size-above-the-rest.md`](../items/BUG-036-the-commit-list-a-size-above-the-rest.md)

## Approach

`font-size: var(--fs-secondary)` on `.message` and `.text` in
`CommitRows.svelte`. Both follow the text scale through the token, as before;
the row pitch is set separately and does not change.

## Files

| File | Change |
| --- | --- |
| `src/lib/graph/CommitRows.svelte` | The two cells' size. |
| `src/lib/ui/flat.test.ts` | The commit list and the file names share the size. |

## Risks and rollback

- More of each message fits, which is the point. Rollback is a revert.
