<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-121 — Sweep

**Item:** [FEAT-121](../items/FEAT-121-what-is-running-is-said-and-what-would-collide-waits.md)

The first four rows were looked at on 2026-10-12 in the built frontend, served with fixed answers in place of the core and drawn by headless Chrome at 1440 by 900. That is the real page and not the real application: git did not run, and the timings were the fixture's.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT121-01 | A repository with uncommitted changes and an upstream | Press Pull, then *Stash and pull* | A capsule over the toolbar says *Stashing your changes…*, then *Pulling…*, then *Putting your changes back…*. Pull shows the strands. The other four buttons and the branch are dimmed. The toolbar does not move. | P1 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT121-02 | As above | Hover a dimmed button during the pull | Its title says *Pulling is still running* | P1 | Titles read from the page against fixtures. Not run in the application. |
| SWEEP-FEAT121-03 | A repository with a remote | Press Fetch | The capsule shows git's progress. Pull and Push are dimmed. Branch, Stash and the branch switcher are not. | P1 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT121-04 | As above | During a fetch, press Stash | The stash is made while the fetch runs | P1 | Not run. |
| SWEEP-FEAT121-05 | Any repository | During a pull, scroll the graph, open a commit's diff, go to Branches | All of it works while the pull runs. Nothing covers the window. | P1 | Not run. |
| SWEEP-FEAT121-06 | A small repository | Switch branch from the toolbar | The switch is over before anything is dimmed or said, and nothing flashes | P2 | Not run. |
| SWEEP-FEAT121-07 | Any repository | During a pull, right-click a commit on the graph and choose Revert | A notice: *Could not revert*, *Pulling is still running*. Nothing is reverted. | P2 | Not run. |
| SWEEP-FEAT121-08 | A push the remote rejects | Press Push | The capsule goes, the buttons come back, and the notice has git's words | P1 | Not run. |
| SWEEP-FEAT121-09 | A window 700 wide | Press Fetch | The capsule is still shown over the toolbar | P3 | Not run. |
| SWEEP-FEAT121-10 | Reduced motion on | Press Pull | The capsule and the strands show, without the weave | P3 | Not run. |
