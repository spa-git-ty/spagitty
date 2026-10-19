<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-057 — Manual sweep

> **Refreshed 2026-10-19 (TASK-073).** TASK-022 took the refracting glass off menus and dialogs. Every ticket here is obsolete; TASK-022's own sweep checks the frost that replaced it.

**Item:** [`agile/items/FEAT-057-liquid-glass.md`](../items/FEAT-057-liquid-glass.md)

**Preconditions for every ticket:** a build of `feature/FEAT-057-liquid-glass`,
a repository open on the Graph screen with enough history that the commit list
fills the window, and — for the tickets that say so — a display whose scale
factor is not 1.

This is the sweep that matters most in the repository, because the whole item is
a thing you can only judge by looking at it. Every ticket below says what to
look *at*, not just what to click.

---

## FEAT-057-T1 — A menu bends the application behind it

**Priority:** high — the item.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Right-click a commit row in the middle of a dense part of the graph | A menu opens over the rows. |
| 2 | Look at the rows immediately outside the menu's edge, top and bottom | They are pushed *outward*, away from the menu — not shifted sideways, not doubled. |
| 3 | Look along the left and right rims | The same push, horizontally. At the corners the two combine into a diagonal. |
| 4 | Look closely at the sharpest part of the bend | A faint colour split at the rim. It should read as glass, not as a printing error. |
| 5 | Look at the middle of the menu, away from the rims | Frosted and still. The bend belongs to the rim; the centre only frosts. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refracting glass off menus and dialogs, which now wear a frost.

---

## FEAT-057-T2 — The ring is on the pane, on a scaled display

**Priority:** high — this is the failure the device-pixel pre-multiply exists for, and it is invisible at a scale factor of 1.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Set the desktop scale to something other than 100% — 125%, 150%, whatever the display offers | |
| 2 | Open a menu and look at where the refraction ring sits | Exactly on the menu's rim, following its corner radius. |
| 3 | Specifically: is the ring up and to the left of the menu? | **No.** That is the map being read as device pixels, and it is the bug this ticket exists to catch. |
| 4 | Move the window to a second display at a different scale, with the menu open | The ring follows within a frame. |
| 5 | Repeat at 100% | Unchanged. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refracting glass off menus and dialogs, which now wear a frost.

---

## FEAT-057-T3 — The window's own edge survives

**Priority:** high — a regression here damages the window, not the effect.

| # | Step | Expected |
| --- | --- | --- |
| 1 | With no menu open, look at the window's outline, its corners and its cast shadow against the desktop | All present. |
| 2 | Open a menu, and look at the same three things | All still present, unchanged. Nothing is clipped, and the corner does not go square. |
| 3 | Close the menu | Unchanged again. |
| 4 | Maximize the window and repeat | The radius and cast are gone by design when maximized; the edge highlight stays. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refracting glass off menus and dialogs, which now wear a frost.

---

## FEAT-057-T4 — A dialog, and a menu over a dialog

**Priority:** high — two panes at once is the case one shared filter exists for.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Trigger a confirmation — a branch delete, or a discard | The dialog refracts the application behind it. |
| 2 | Confirm it is still centred, and the backdrop still dims | Yes to both. The dialog must **not** have moved to a corner. |
| 3 | With the dialog open, raise a menu over it if the dialog offers one | Both panes refract. Neither doubles the other where they overlap. |
| 4 | Watch the dialog while the menu opens and closes | The dialog does not change appearance. The thickest pane sets the material, and a tie keeps the first. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refracting glass off menus and dialogs, which now wear a frost.

---

## FEAT-057-T5 — It lets go, and it costs what it should

**Priority:** medium — this window rasterizes on the CPU (FEAT-055), so an effect left running is a tax on everything.

| # | Step | Expected |
| --- | --- | --- |
| 1 | With no pane open, inspect `.lens` in the web inspector | No `filter` property. Not an identity filter — none at all. |
| 2 | Look for the `<svg>` filter host in the body | Absent while nothing is open. |
| 3 | Open a menu, then close it, five times | The host appears and disappears each time. Nothing accumulates: one host, one `<filter>`, never two. |
| 4 | Scroll the graph hard with a menu open, then with none | Note both. A visible difference is expected; an unusable difference is a finding — record the numbers. |
| 5 | Resize the window with a menu open | The ring keeps up. Some lag is expected; a ring that detaches and lands late is a finding. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refracting glass off menus and dialogs, which now wear a frost.

---

## FEAT-057-T6 — Reduced motion and the theme

**Priority:** medium — the effect must not be the one thing that ignores the user's settings.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Turn on the desktop's "reduce motion" setting and open a menu | The menu still refracts — refraction is not motion — but nothing animates into place. |
| 2 | Switch the theme in Settings → Appearance, with a menu open | The glass takes the new palette. The rim does not stay the old theme's colour. |
| 3 | In the light theme, check the rim highlight against the pane | Still readable. Thin glass in a light theme is where this reads cheapest. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refracting glass off menus and dialogs, which now wear a frost.

---

## FEAT-057-T7 — A second machine

**Priority:** low, and it is the ticket that ages best.

| # | Step | Expected |
| --- | --- | --- |
| 1 | Run T1 on a host with a different GPU driver and compositor | Record the driver, the compositor and the distribution in the result field. |
| 2 | Look for a fringed band anywhere outside a pane | None. That is `feDisplacementMap` reading past the edge of a map, and it would mean the filter region and the maps have come out of step. |
| 3 | Confirm the refraction happens at all | A WebKit version where `feImage` with a data URI behaves differently would show no bend rather than a wrong one. Say which it is. |

**Result:** Obsolete, 2026-10-19 — TASK-022 took the refracting glass off menus and dialogs, which now wear a frost.
