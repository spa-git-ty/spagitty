<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-042 — Manual sweep

**Item:** [`agile/items/TASK-042-one-control-scale-and-a-platform-policy.md`](../items/TASK-042-one-control-scale-and-a-platform-policy.md)

**Tickets 5 to 9 need a Mac and have not been run.** Everything about the macOS
window here is what Tauri documents plus a frontend that agrees with it; none of
it has been seen. That is the honest state and the reason those tickets are
written as carefully as they are.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | Any platform | Hover slowly across a dialog's row of buttons, then down the nav rail from Farm to Settings | **Nothing moves.** The fill and the border change under the pointer and the row stays where it is. Before this, every rail row slid two pixels sideways as the pointer crossed it — along the axis the pointer was travelling. | High | |
| SWEEP-002 | Any platform | Settings → text size 130%. Look at the title bar, the toolbar's branch name, a badge and the repositories hero | All of them grow. These were the last eight fixed sizes in the application; a 9px badge pin in particular should now be readable. | High | |
| SWEEP-003 | Text 130%, window at ~900px | Look at the title bar and the Badges screen | The name still centres, the window controls are not clipped, and the badge grid does not overlap. This is where growing eight sizes can reflow something. | High | |
| SWEEP-004 | Any platform | Use the application normally for ten minutes, then turn the hover lift back on locally and use it again | A judgement, not a check: does the interface feel steadier without it? The review says it should. Record disagreement — this is a taste change and it is not behind a preference. | Medium | |
| SWEEP-005 | **A Mac**, a build from this branch | Launch it | The window has macOS traffic lights at the **top left**, and **no** Spagitty window controls at the right. If it has neither, the platform config did not merge and that is the first thing to check. | High | |
| SWEEP-006 | The same | Look at the title bar's left end | The traffic lights sit in the reserved 78px and overlap nothing. The name "Spagitty" is centred in the window, not pushed right. | High | |
| SWEEP-007 | The same | Drag the window by the empty part of the title bar; double-click it; drag each edge and corner to resize | Dragging moves it, double-click zooms it, and every edge resizes using **the system's** handles. Spagitty draws none of its own there any more, so a dead edge is a finding. | High | |
| SWEEP-008 | The same | Enter full screen (green button, and ⌃⌘F) | It goes full screen; the traffic lights hide; the 78px reservation becomes dead space at the left of the bar. **Record how bad that looks** — if it is noticeable, the reservation needs to follow the full-screen state and that is a follow-up. | High | |
| SWEEP-009 | The same | Check the window's corner and shadow, and Cmd+F, Cmd+P, Cmd+= | The frame is macOS's own, with no second card drawn inside it. Shortcuts work and are labelled `⌘`. | High | |
| SWEEP-010 | Linux, Omarchy tiled | Launch it | Unchanged: flush window, Spagitty's own controls at the right, custom resize edges working. The macOS work must be invisible here. | High | |
| SWEEP-011 | Windows | Launch it | Unchanged: the drawn card with its corner and shadow, Spagitty's own controls, custom resize edges. Windows is the only platform still undecorated. | High | |
| SWEEP-012 | Any platform, reduced motion enabled | Press a button and a chip | They still respond — the press motion is suppressed by `app.css`'s reduced-motion block, and the colour change remains. A control with no feedback at all would be a regression this task could cause. | Medium | |

## Negative paths this sweep deliberately covers

- **SWEEP-010 and SWEEP-011** are the platforms this change must not touch, and
  the whole risk of a platform-specific config is that it stops being specific.
- **SWEEP-008** is the case the 78px reservation is most likely to get wrong,
  and it is written to record rather than to pass or fail, because the right
  answer depends on how it looks.
- **SWEEP-012** is the interaction between two changes: hover motion was removed
  and press motion kept, and press motion is exactly what reduced-motion already
  suppresses. A control that now does nothing at all under both is a control
  with no feedback.
