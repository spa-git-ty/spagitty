<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-054 — Plan

**Item:** [`agile/items/BUG-054-review-loader-conversation-panel-and-scrollbars.md`](../items/BUG-054-review-loader-conversation-panel-and-scrollbars.md)

`ReviewInbox` and `ReviewRoom` use `Loader`. `RoomConversation` takes `onhide`; its head wraps. `ReviewRoom` keeps `conversationHidden` in `localStorage` and draws `.conversation-tab` in the card's place. `src/lib/ui/scrolling.ts` `watchScrolling`, mounted by the layout; `app.css` thumbs transparent unless `.is-scrolling` or hovered (and the standard-property fallback the same way).
