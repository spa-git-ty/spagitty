<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-029 — Manual sweep

**Item:** [`agile/items/BUG-029-the-window-is-ringed-with-blurred-desktop.md`](../items/BUG-029-the-window-is-ringed-with-blurred-desktop.md)

This sweep is the verification. The defect is a compositor compositing a window
Spagitty asked it to composite, and nothing in the test suite can see it — the
automated tests cover every decision that leads to the pixels and none of the
pixels. `SWEEP-001` and `SWEEP-002` are what close this item.

**Two of these need hosts this project is not developed on.** `SWEEP-005` and
`SWEEP-006` guard the half of the change that must not have moved, and until
somebody runs them the claim "nothing changed on macOS and Windows" is an
argument rather than an observation.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | Hyprland with blur on — Omarchy's default — and a release AppImage built from this branch | Run the AppImage. Look at all four edges of the window against a busy wallpaper | The content meets the compositor's border. No band of blurred desktop anywhere, and no second rounded corner inside Hyprland's own. | High | |
| SWEEP-002 | The same window | Compare against the screenshot on the item document | The ~10px inset is gone. This is the before-and-after the bug was reported with. | High | |
| SWEEP-003 | The same window, tiled on a workspace with another window | Toggle it between tiled and floating (`SUPER+V` on Omarchy), then fullscreen and back | Flush in every state. This is the case the `maximized` branch missed: a tiled window is not maximized, and restoring one must not hand the gap back. | High | |
| SWEEP-004 | A Linux session that is **not** Hyprland — GNOME or KDE | Run the AppImage | Flush, and the desktop's own shadow and corner around it. Plainer than macOS, and correct: nothing is drawn twice. | Medium | |
| SWEEP-005 | macOS, a build from this branch | Open it, look at the corner and the shadow, then maximize and restore | Unchanged from before this branch: the floating card with its rounded corner and shadow, square when maximized. | High | |
| SWEEP-006 | Windows, a build from this branch | The same | The same. Unchanged. | High | |
| SWEEP-007 | Any host | Drag the window by its title bar; resize it from each edge and corner | Both still work. The window is undecorated on every host and supplies its own drag region and resize edges; `ResizeEdges` is `position: fixed` and does not depend on the gap, but that is an argument until somebody drags it. | Medium | |
| SWEEP-008 | Linux, the running window | Watch a scroll of a large graph, and a theme change | No flicker or tearing at the window edge from the window now being opaque. Opacity was chosen partly for the frame cost; it must not have bought a different artefact. | Low | |

## Negative paths this sweep deliberately covers

- **SWEEP-004** is the case the fix is least confident about. The user agent
  says "Linux", not "this compositor decorates windows", and on a bare
  compositor that draws nothing the result is a plain rectangle with no shadow.
  That is the accepted cost, recorded in the item's non-scope — but somebody
  should look at it and say whether "plain" is what they see or whether it is
  worse than that.
- **SWEEP-005 and SWEEP-006** are the regression half. The change is a branch
  added to a decision two other platforms already relied on, and the way this
  goes wrong is not on the platform it was written for.
- **SWEEP-003** covers the actual mechanism of the original miss, rather than
  its symptom.
