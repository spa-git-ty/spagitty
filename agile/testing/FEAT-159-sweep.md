<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-159 — Sweep

**Item:** [FEAT-159](../items/FEAT-159-clear-all-notifications.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT159-01 | Notifications from two repositories | Bell › Clear all › Clear all | The drawer is empty and stays empty after a restart | Not run; the author is checking it |
| SWEEP-FEAT159-02 | As above | Pick *This repo*, then Clear all | Only this repository's go; *All repos* still lists the other's | Not run |
| SWEEP-FEAT159-03 | Some notifications | Clear all, then Cancel | Nothing changes | Not run |
