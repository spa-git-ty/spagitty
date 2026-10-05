<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-047 — Plan

**Item:** [`agile/items/BUG-047-whole-file-scrolls-back-to-the-top.md`](../items/BUG-047-whole-file-scrolls-back-to-the-top.md)

## Approach

The immediate jump in `scrollToIndex` runs under `untrack`. The second jump, a
frame later, already runs outside any effect. Fixed in the list rather than in
the room, so no other caller of `VirtualRows` can fall into it — TASK-052's
plan reuses the component.

## Files

| File | Change |
| --- | --- |
| `src/lib/ui/VirtualRows.svelte` | `scrollToIndex` jumps untracked. |
| `src/testing/VirtualRowsHarness.svelte` | A list asked for its top row from an effect. |
| `src/lib/ui/VirtualRows.test.ts` | Measuring a row neither re-runs that effect nor moves the reader. |

## Risks and rollback

- A caller that relied on being re-run as rows are measured loses that. None
  did: the room's two callers want one jump each. Rollback is a revert.
