<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-116 — Plan

**Item:** [FEAT-116](../items/FEAT-116-pull-requests-does-not-review.md)

Remove every review control from `PRWorkspace`, `PRDiffPane` and `RequestDetail`, then the store code only they used, then the CSS they left behind, and the tests of the removed paths. Add tests that the store has none of the removed members and that the screen renders none of the removed controls.
