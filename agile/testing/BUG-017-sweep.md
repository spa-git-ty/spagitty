<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-017 — Manual sweep

**Item:** [`agile/items/BUG-017-the-lens-wipes-the-window.md`](../items/BUG-017-the-lens-wipes-the-window.md)

**Preconditions for every ticket:** a build of
`bugfix/BUG-017-the-lens-wipes-the-window`, and a repository open. T1 needs the
window's `devicePixelRatio` to be something other than 1 — read it in the Web
Inspector; do not take the compositor's monitor scale for it, which is what
misled the diagnosis. On the reporting machine Hyprland says `scale: 1` and the
webview says 1.3636.

---

## BUG-017-T1 — The reported failure is gone

**Priority:** high — this is the report.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Confirm `devicePixelRatio` is not 1 | Record the value. At 1 this ticket proves nothing — the old code was correct there. |
| 2 | Open the branch dropdown in the toolbar | The menu opens. |
| 3 | While it is open, look at the right-hand edge of the window | The tab strip, the toolbar and the screen reach the edge. No flat band of background down the right. |
| 4 | Look at the bottom of the window | The status strip is drawn, in full, across the whole width. |
| 5 | Close the menu | Nothing changes about the window — it was already whole. |
| 6 | Repeat at two more window sizes, one much smaller | The same at every size. The old fault scaled with the window, so one size is not a test. |

**Result:**

---

## BUG-017-T2 — Menus do not pile up

**Priority:** high — the second half of the report.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open the branch dropdown, close it with Escape | The menu is gone. Nothing is left drawn where it was. |
| 2 | Open it again, close it by clicking elsewhere | Same. |
| 3 | Right-click a commit, close it; right-click a different commit further down | One menu on screen, in the new place. No trace of the first. |
| 4 | Do that ten times quickly, in different places | Still one menu at a time, and a clean window after the last is closed. |
| 5 | Open a menu, then a dialog over it | Both are glass, both are whole, and the window behind them still paints to its edges. |

**Result:**

---

## BUG-017-T3 — The glass still looks like glass

**Priority:** high — the fix rewrote the geometry the effect is made of.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open a menu over the commit list | The frost sits **on the menu's own footprint** — its edges line up with the menu's edges, not a larger or smaller rectangle offset from it. |
| 2 | Look at the rim | The backdrop bends outward around the edge, and the colour splits slightly where the bend is sharpest. The ring is on the rim, not up and to the left of it. |
| 3 | Compare against the frost before this change | Noticeably thicker — `blur` went from 13 to 28. Text behind the menu should read as colour and shape, not as words. |
| 4 | Open a menu near the bottom edge, and near the right edge | Measured and moved inside the window as before, with the frost following it. |
| 5 | Switch to another theme, including a light one, and repeat step 1 | The glass reads as glass in each. The shadow is the theme's ink, not black. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refraction off menus; the frost stays, and T1, T2, T4 to T6 still apply.

---

## BUG-017-T4 — Across ratios and displays

**Priority:** high — the ratio is the whole fault, and it is now meant to be irrelevant.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Open a menu on the usual display | Whole window, correct frost. |
| 2 | Move the window to a display at a different scale, if one is available, and open a menu | The same. Nothing about the region depends on the ratio now. |
| 3 | Open a menu, and while it is open resize the window from a corner | The window paints to its edges throughout, and the frost stays on the menu. |
| 4 | Close the menu, resize the window, open a menu again | Correct at the new size — a resize with no menu open is not observed by the lens, so this checks the rebuild on the way back in. |
| 5 | Change the interface zoom (Ctrl `=`, Ctrl `-`) with a menu open and closed | No wipe at any zoom. |

**Result:**

---

## BUG-017-T5 — Everything the lens touches

**Priority:** medium — every menu and dialog raises it, not just the toolbar's.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Right-click a repository tab | Menu, glass, whole window. |
| 2 | Right-click the graph column header | Same. |
| 3 | Right-click a branch on the Branches screen | Same. |
| 4 | Trigger a confirmation dialog — a history rewrite will do — and cancel it | The dialog is glass and centred, and the window behind it is whole. |
| 5 | Open the command palette | Unchanged: it is not a glass pane and should not have become one. |

**Result:**

---

## BUG-017-T6 — On a display at ratio 1

**Priority:** medium — the negative case, so the fix is not a swap of one fault for another.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Run on a display where `devicePixelRatio` is exactly 1 | Confirm the value first. |
| 2 | Open a menu | Whole window. |
| 3 | Look at the frost | On the menu's footprint, the same size as the menu — the old code was correct here, and this must still be. |

**Result:**
