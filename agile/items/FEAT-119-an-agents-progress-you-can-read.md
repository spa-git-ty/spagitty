<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-119 — An agent's progress you can read

**Status:** Done — on `feature/FEAT-119-an-agents-progress-you-can-read`; unmerged. Not yet looked at in the running app.
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

## What was done

- **Now, in words.** While an agent works the card says the step it is on and how long that step has taken (*avatars.rs · 2 findings · 0:42*), under the sentence.
- **Progress is the step list.** The bar counts files or conflicts once the plan has said how many (*1 / 4 · 1:20*), and steps before that (*1 / 3 steps · 1:02*), or only the time while nothing says how many there will be. `levels.ts` has `progressOf`, `currentStep` and `nowLine`; `meter` and `share` use them.
- **Quiet is not working.** After three minutes of silence the card says *Nothing from <agent> for N min* with a *Stop* button, and the sentence takes the warning colour. When the agent speaks again, the engine puts back the sentence it replaced and clears `quiet_since` (`Inner::heard`, which it did not do before). The inbox and Merger cards say *quiet for N min* too (`cardLine`).
- **The pulsing dot is gone.** The sentence is words; a paused run has its own tone.
