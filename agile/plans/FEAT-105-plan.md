<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-105 — Plan

**Item:** [`agile/items/FEAT-105-review-from-the-pull-request-screen.md`](../items/FEAT-105-review-from-the-pull-request-screen.md)

`PRWorkspace.svelte` gains `openInReview`: `review.open(request)` (FEAT-087's store, which keys the room and reads its record), then `goto('/review')`; a failure is a notice. A primary **Review** button heads the actions, and the developer footer gets **Reply in Review**.
