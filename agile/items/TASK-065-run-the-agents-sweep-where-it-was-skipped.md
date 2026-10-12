<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-065 — Run the agents sweep that FEAT-115 was closed without

**Status:** Backlog — raised by the TASK-063 audit, 2026-10-12.
**Screens:** 1K, 1R, 1S.
**Raised by:** the audit of pull request #52.

## Why

FEAT-115 is marked Done. Its sweep (`agile/testing/FEAT-115-sweep.md`) has eight P1 and P2 tickets, and seven of them say *Not run*: none was run on an installed agent, with a provider key, or in a native window, on any of the three systems. Within days the author reported BUG-065, BUG-070, BUG-071 and, from the audit, BUG-072: each a way a real agent did not work, of the kind a first real run of tickets 02 to 05 is meant to find.

## Acceptance criteria

- SWEEP-FEAT115-02 to 08 are run on Windows, macOS and Linux, with at least Claude Code, Codex, one API agent and a local OpenAI-compatible server, and their results written in the sweep file.
- Each failure becomes a bug item.
- The author decides whether FEAT-115 keeps *Done*, goes to *Partial*, or whether a feature may be *Done* with a P1 sweep ticket not run. The recommendation is that it may not.

## For the agent who picks this up

**Who:** unassigned. It needs the author's machine, provider keys and the agents installed; an agent without them can only prepare.

- BUG-070, BUG-071 and BUG-072 name the tickets that belong to them; run those with this.
