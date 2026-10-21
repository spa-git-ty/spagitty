<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-157 — Sweep

**Item:** [FEAT-157](../items/FEAT-157-close-a-terminal-session-and-give-tmux-our-colours.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-FEAT157-01 | Terminal with psmux, two sessions | Press × on the other session's tab, then Cancel | Asked first; Cancel leaves it running | Not run; the author is checking it |
| SWEEP-FEAT157-02 | Same | Press *Close* in the header, then Close | The session ends and its tab goes; the other is shown | Not run |
| SWEEP-FEAT157-03 | Every session closed | Press `+` | The repository's session is made again | Not run |
| SWEEP-FEAT157-04 | A `~/.psmux.conf` with its own colours | Open Terminal | The status line, borders and messages are in Spagitty's colours, the session name on the accent; the config's key bindings still work | Not run |
| SWEEP-FEAT157-05 | Terminal open | Change the accent, then light and dark | The terminal and the psmux bars change with it | Not run |
