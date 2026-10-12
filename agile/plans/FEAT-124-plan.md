<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-124 — Plan

**Item:** [FEAT-124](../items/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top.md)

## Steps

1. **The words, as functions.** In `levels.ts`: `isWorking(a, now)` for when anything may move, and `askOf(a, names)` for what waits: the step, its undecided proposals, one line, and the name of what opens under the line or null. One case per kind of stop, each with a test, as the rest of that file is.
2. **The card.** The sentence and what it asks become one block at the top. While the agent waits the block is the tinted card the timeline used to draw under the waiting step, with the same buttons. The primary one is first. The timeline keeps the steps and the reason a run ended.
3. **What opens.** One piece of state: the index of the step whose question is open. A new stop is a new step, so it starts closed with no effect to reset it.
4. **The motion.** A global class in `app.css`, because three components use it. The words are painted by a gradient clipped to the text. The band is the agent's colour and the rest is the text's own colour, faded.
5. **Where else it shows.** The inbox card's agent line and the room's *Agent* tab.

## What decides it is right

- The buttons are above the meter and the timeline in the document, and none is left in the timeline.
- Nothing moves in any state but working and heard from.
- With the motion off the words are fully legible: the gradient is removed, not frozen part-way.
- When the question is open in a short card, the timeline gives way before the card overflows.
