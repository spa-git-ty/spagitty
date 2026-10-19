<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-095 — Plan

**Item:** [BUG-095](../items/BUG-095-the-terminal-s-plus-says-no-server-running.md)

1. Reproduce the message with the shipped psmux: `new-window` to a session that was never made, then the same after `new-session -d`.
2. In `terminal.add`, when the session shown has no terminal entry (its client exited), call `attach`, whose `new-session -A` makes the session, instead of `act({ kind: 'newWindow' })`.
3. A store test: the client exits, `+` opens a client on the same session and asks for no window.
