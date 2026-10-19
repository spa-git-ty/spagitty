<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-146 — A conflict resolver chosen in Rules

**Status:** Backlog.
**Screens:** 1Q, 1S.
**Proposal:** [A local hub for parallel agents](../../design_handoff_home_farm_notifications/HUB.md), step 5.

## Change

When the landing lane (FEAT-144) meets a conflict, the farm's Rules choose who
resolves it: an agent, a tool, or stop and ask (today's behaviour). The
resolution is made through Merger's entry point, so a resolved conflict looks
the same whoever resolved it.

## Questions for the author

From `HUB.md`: which agent resolves by default, and does a resolved conflict
need a second review?
