<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-056 — Plan

**Item:** [BUG-056](../items/BUG-056-review-requests-and-profile-navigation.md)

Finish the interrupted changes already in the checkout. `ScreenHead` and `EmptyState` supply the shared presentation for `ReviewInbox` and the Pull requests page. Keep the current repository scope on inbox mount. The status-strip profile menu calls `goto('/settings#you')`.

Update mounted-screen tests for the repository pill, removed scope controls, scope reset, empty-state recovery and preserved host errors. Move the existing cross-repository behavior checks to the review store tests so removing a screen control does not remove coverage of supported store behavior. Add a menu-click regression for profile navigation and session preservation.

Run focused tests, the complete frontend suite, type checks and the production build. Inspect the headers and window-centred empty states in a browser and record the result. No dependencies are added.
