<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-142 — Sweep

**Item:** [FEAT-142](../items/FEAT-142-a-notification-center.md)

None of these rows has been run in the application. The author is doing the visual checks.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT142-01 | Any screen | Look at the title bar, then click the bell | The bell is left of the window controls. The drawer opens from the right under the title bar, 470 px wide; the rest of the window dims. The bell looks pressed. | P1 | Not run. |
| SWEEP-FEAT142-02 | Drawer open | Press Escape; open it again and click the dimmed window | Each closes it. | P1 | Not run. |
| SWEEP-FEAT142-03 | Offline | Fetch | The toast says it failed as before, and the drawer gains *Fetch failed* under Git with git's words. *Fetch again* fetches. | P1 | Not run. |
| SWEEP-FEAT142-04 | Pull request notifications on | Wait for a review request | The toast and the system notification as before; the drawer keeps it with the headline verbatim and *Review*. | P1 | Not run. |
| SWEEP-FEAT142-05 | A farm at Semi-automatic | Let a task pass review | *T-n is ready to merge* under Farm; *Approve merge* opens Merger on it. | P1 | Not run. |
| SWEEP-FEAT142-06 | A branch whose last run passed | Push a commit that fails its checks | *Checks failed* on the next look; after a green run, *Checks passing again*. | P2 | Not run. |
| SWEEP-FEAT142-07 | Entries from two repositories | Switch All repos / this repo; search; each chip | The list follows; the Unread chip carries its count; an empty chip says *Nothing here*. | P2 | Not run. |
| SWEEP-FEAT142-08 | Several days of entries | Scroll | Today, Yesterday, then dated groups; their headers stick. | P2 | Not run. |
| SWEEP-FEAT142-09 | Some entries | Restart Spagitty | Everything is still there, read ones still read. | P1 | Not run. |
| SWEEP-FEAT142-10 | Entries older than 90 days | Settings › Notifications › Clear older than 90 days | They go; the rest stay. | P2 | Not run. |
| SWEEP-FEAT142-11 | Light theme | Open the drawer | Legible; tiles and the unread dot follow the palette. | P2 | Not run. |
