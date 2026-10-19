<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-145 — History as tools for agents

**Status:** Backlog.
**Screens:** 1Q.
**Proposal:** [A local hub for parallel agents](../../design_handoff_home_farm_notifications/HUB.md), step 4.

## Change

Blame, a path's history and search, offered to the farm's agents over MCP and
cached per commit, so an agent asks the hub instead of walking history itself.
The agents Spagitty calls by API, which have no folder, would read files only
through these tools.

## Acceptance criteria

To be written when the work begins. Recorded so the hub's order stays visible:
slices (FEAT-143) and the landing lane (FEAT-144) come first.
