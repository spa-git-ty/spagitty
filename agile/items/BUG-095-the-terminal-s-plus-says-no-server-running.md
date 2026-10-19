<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-095 — The Terminal's `+` says *no server running on session*

**Status:** Fixed — on `release/3.4.0`; unmerged. Covered by a store test; not tried in the running application (the author is checking it).
**Screens:** 1T (Terminal)
**Raised by:** the author, on 3.0.0, with a screenshot of the Terminal: psmux's *no server running on session*. Left open by the 3.1 review; recorded and fixed 2026-10-19.

## What happens

With a multiplexer, `+` asks it for a new window in the session shown. The screen keeps showing a session after the client Spagitty attached to it has exited: when the last shell in the session ends, the session and its server end with it, and the client goes. `+` then asks for a window in a session that is not there, and psmux answers *no server running on session '<socket>__<session>'*.

Reproduced with the psmux Spagitty ships (3.3.8): `psmux -L spagittytest new-window -t "foo-abc123:"` with no session prints exactly that and exits 1; with the session made first, the same command adds a window. The moment in the application where the author met it was not seen live.

## Acceptance criteria

- `+` on a session with no client of Spagitty's attaches to it again, which makes it, instead of asking for a window.
- `+` on a session with a client still asks for a window, as before.
- A store test covers the first.

## Fix

`terminal.add` in `src/lib/terminal/store.svelte.ts` attaches again when the session shown has no terminal of its own.
