<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-153 — Plan

**Item:** [FEAT-153](../items/FEAT-153-blame-on-click-can-be-turned-off.md)

1. `store.svelte.ts`: `blameOnClick`, on by default, remembered as `click` beside `blame` and `wrap`; `toggleBlameOnClick()` closes an open peek when it turns off.
2. `FileView.svelte`: the breadcrumb sits in a `.head` row with the toggle at its right, shown for text files. `pressLine` moves the cursor and stops there while the toggle is off.
