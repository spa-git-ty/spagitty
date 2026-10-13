<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-135 — Farm sessions in the terminal

**Status:** Backlog — recorded 2026-10-13, split from FEAT-130. Not started, and not in 3.0.0.
**Branch:** `feature/FEAT-135-farm-sessions-in-the-terminal`.
**Screens:** 1T (Terminal: the Sessions panel), 1A (Farm: where a local agent is run).
**Raised by:** the author, 2026-10-13, answering the proposal's question 5: farm sessions in the terminal come later, as their own item. The terminal and the multiplexer are in 3.0.
**Proposal:** [`docs/proposals/workbench-and-recipes.md`](../../docs/proposals/workbench-and-recipes.md) §5.3. **Designs:** [`design_handoff_workbench/`](../../design_handoff_workbench/) — `Main.dc.html`, the two agent rows in the Sessions panel.

## Change

The farm may run a local agent inside the multiplexer FEAT-130 brought, on Spagitty's socket, as the session `farm/<task>`. Terminal lists those sessions and lets the person watch one.

## Acceptance criteria

- **Where an agent runs**: when a multiplexer is in use, the farm starts a local agent in `tmux -L spagitty` (psmux on Windows) as `farm/<task>`. With none, the farm runs it as it does today.
- **The Sessions panel** lists farm sessions apart from the repository's own, each with the agent's hexagon, its task and its state.
- **Read-only**: a farm session is attached with `attach -r`. Keys typed in it go nowhere, and the screen says the session is being watched. To type, the person takes over in Farm.
- ***Follow repository*** never switches to a farm session.
- **Closing Spagitty** detaches and never ends an agent's session.
- **Tests**: Rust tests for the session's name and the read-only attach; component tests for the panel's farm rows.

## Not in this item

- **Typing into an agent's session from Terminal.**
- **Agents on another machine.**
