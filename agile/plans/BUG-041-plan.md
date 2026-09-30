<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-041 — Plan

**Item:** [`agile/items/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow.md`](../items/BUG-041-the-pinned-header-cuts-the-detail-cards-shadow.md)

## Approach

Move `background-color: var(--bg)` from `.header-frozen` to
`.header-frozen.scrolled`, which already carries the seam's shadow and already
follows `scrollLeft > 0`, and transition it with the shadow.

Diagnosed in the release build: the element under each point beside the card
was walked for the first painted background, and only `.header-frozen` was
opaque. The rule was then injected into the running build, and the pixel at
the strip changed from `(251, 247, 241)` to `(249, 245, 239)`, the tint of the
pane beside it.

## Files

| File | Change |
| --- | --- |
| `src/lib/graph/GraphHeader.svelte` | The background, while scrolled only. |
| `src/lib/ui/flat.test.ts` | Held. |

## Risks and rollback

- None beyond the one rule. Rollback is a revert.
