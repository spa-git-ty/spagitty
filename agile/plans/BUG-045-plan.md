<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-045 — Plan

**Item:** [`agile/items/BUG-045-the-name-leaves-the-title-bar.md`](../items/BUG-045-the-name-leaves-the-title-bar.md)

## Approach

Drop the `{#if}` around the name and the `tabbed` grid that gave the middle
column no width. The bar is TASK-021's three columns again, outer two equal:
tabs on the left, the name centred, the controls on the right. The tab strip
already scrolls sideways when it runs out of room.

## Files

| File | Change |
| --- | --- |
| `src/lib/chrome/TitleBar.svelte` | The name always drawn; the `tabbed` variant gone. |
| `src/lib/chrome/chrome.test.ts` | The FEAT-082 test now expects the name with a tab open. |

## Risks and rollback

- The tabs get half the row less half the name, not the whole of it less the
  controls: about five tabs at 1600 px before the strip scrolls. Rollback is a
  revert.
