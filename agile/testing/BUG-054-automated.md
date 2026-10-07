<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-054 — Automated test record

**Item:** [`agile/items/BUG-054-review-loader-conversation-panel-and-scrollbars.md`](../items/BUG-054-review-loader-conversation-panel-and-scrollbars.md)

| Suite | Cases |
| --- | --- |
| `src/routes/review/page.test.ts` | The loader while the first list is read; the Conversation card hides into a tab, remembers it, and comes back. |
| `src/lib/ui/scrolling.test.ts` | Only the scrolling element is marked, for the linger after its last scroll; stopping clears every mark; the stylesheet paints a thumb only when marked or hovered. |
