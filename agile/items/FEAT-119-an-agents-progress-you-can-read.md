<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-119 — An agent's progress you can read

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** 1R, 1S.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

While an agent works, its progress is mostly a pulsing dot beside its sentence (`.sentence.working .state`, `animation: breathe`, in `src/lib/agents/AgentCard.svelte`). The meter under it only moves once there is a plan with files or conflicts, and stays empty while the agent reads, plans or runs checks. A slow agent and a stuck one look the same.

## Change

- The card says what the agent is doing now, in words: the step's name, the file or conflict it is on, and how long that step has taken.
- Progress is the step list: done, current and to come. The bar counts files or conflicts when they are known, and steps when they are not.
- When nothing has come from the agent for a while (`quietSince` is already kept), the card says so, with how long, and offers *Stop*.
- The pulsing dot goes. Motion is kept for something that changed, not for "still working".

## Acceptance criteria

- At any moment of a run, the card says which step the agent is on and how far through the run it is.
- An agent that has said nothing for longer than a set time is shown as quiet, not as working.
- Component tests cover each life state (`starting`, `working`, `waiting`, `paused`, `stopped`, `failed`, `done`).

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/agents/AgentCard.svelte`, `src/lib/agents/levels.ts` (`meter`, `share`, `planned`, `cardLine`), `src/lib/review/InboxCard.svelte` and the Merger result card, which show the same line.
- Do it together with FEAT-118.
- **Branch:** `feature/FEAT-119-an-agents-progress-you-can-read`, with plan, testing documents and a changelog entry when the work starts.
