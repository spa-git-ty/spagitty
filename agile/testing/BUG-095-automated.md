<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-095 — Automated test record

**Item:** [BUG-095](../items/BUG-095-the-terminal-s-plus-says-no-server-running.md)

- `src/lib/terminal/store.test.ts`, *attaches again on `+` when the session's client has gone*: after the client exits, `+` opens a second client on `thing` and sends no `newWindow`.
- The existing *asks the multiplexer for a window, a split and to be let go* still passes: with a client, `+` is a window.

## Results — 2026-10-19

- `bunx vitest run src/lib/terminal`: 92 passed.
- By hand on Windows with the shipped psmux 3.3.8, on a socket of its own (`-L spagittytest`): `new-window` to a missing session printed *no server running on session 'spagittytest__foo-abc123'* and exited 1; after `new-session -d -s foo-abc123`, `new-window` exited 0 and the session listed three windows. The test server was killed afterwards.
