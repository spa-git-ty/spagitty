<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-028 — Manual sweep

## STYLES-01 — Chrome and sparse screens fill the window

**Priority:** High.
**Preconditions:** Development app, repository with an empty Farm task list.

1. Restart the dev server with the app previously open, then reload the app.
2. Open Graph and Farm; inspect title, window controls and toolbar alignment.
3. Resize the window larger and smaller; maximize and restore it.
4. Inspect Farm with no tasks, then navigate to Settings and back.

**Expected:** Title and toolbar remain centered, controls stay aligned, app
background fills the window within its intended shadow margin, status strip
stays at the bottom, and pane scrolling stays inside the app.
**Pass/fail:** Core visual checks passed by the author on 2026-09-05: "both
fixed" confirmed centered chrome and Farm background filling the window after
the server reloaded. Resize/maximize and the extended navigation sweep remain
unverified; no native automation was available.

## STYLES-02 — Hot updates retain layout

**Priority:** Normal.
**Preconditions:** Development app with the fix applied.

1. Make a reversible CSS edit to a component currently shown.
2. Observe the update, restore the edit, and navigate away and back.

**Expected:** Updated styles appear and restore without unstyled chrome or
content-dependent window height.
**Pass/fail:** Pending.
