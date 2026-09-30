<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-083 — Plan

**Item:** [`agile/items/FEAT-083-the-content-in-the-spatial-language.md`](../items/FEAT-083-the-content-in-the-spatial-language.md)

## Approach

**Two tokens, redefined on `.pane`.** Seventeen components paint their header
bands in `--chrome-veil`, and the graph paints its column in `--graph-bg`.
Redefining both on the pane changes all of them at once and none of them
outside it — the ornaments and dialogs keep their own values.

**The Graph's rows keep their geometry.** Rows are absolutely positioned at
`left: 0; right: 0`, and the lane canvas and the bed beneath are laid out from
the same column arithmetic. So the capsule is a `::before` inset inside the
row at `z-index: -1` — under the cells and the canvas, above the scroller's
own background — rather than a narrower row. A row's own slice of the graph
band becomes clear, since the bed already paints it and an opaque slice would
cut the capsule.

**The pinned pane is opaque only when it has to be.** `.rows.scrolled` (the
same `moreLeft` the scroll edges already use) turns the pinned cells' pane
colour on, and the seam's shadow with it; the header's pinned half follows
`scrollLeft > 0`.

**Cards.** The working-copy row and the detail panel take the ornaments'
edge, `--pane-edge` with a lit top, and their own corners. The detail card's
margin sits inside the width the splitter sets.

## Files

| File | Change |
| --- | --- |
| `src/app.css` | `--chrome-veil` and `--graph-bg` on `.pane`. |
| `src/lib/graph/CommitRows.svelte` | Capsule highlight; clear row band; pinned cells and seam only while scrolled; no column rules; working-copy card. |
| `src/lib/graph/GraphHeader.svelte` | Muted labels; dividers only on hover; pinned half's seam only while scrolled. |
| `src/lib/graph/CommitDetail.svelte` | The inspector card. |
| `src/lib/ui/RefChip.svelte` | Capsules; the tag's half-capsule. |

## Risks and rollback

- **Every screen's header loses its fill.** Where a screen relied on the band to
  separate a sticky header from content scrolling under it, the content would
  show through. None of the seventeen is sticky over scrolling content; the
  graph's pinned header half is given the pane colour explicitly.
- Rollback is a revert.
