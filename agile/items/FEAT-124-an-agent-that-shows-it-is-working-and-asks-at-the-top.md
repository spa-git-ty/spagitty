<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-124 — An agent that shows it is working, and asks at the top of its card

**Status:** Done — on `feature/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top`; unmerged. Seen in the development page against fixtures, drawn by headless Chrome; not yet in the running app.
**Screens:** 1R, 1S.
**Raised by:** the author, 2026-10-12, on 2.2.0, with two pictures of the review room. The pictures did not reach the session that built this; the request was read from its words and from the code.

## Problem

Two things, both on the agent card (`src/lib/agents/AgentCard.svelte`).

1. **Nothing says the agent is working.** After an agent is assigned to a review the card shows a sentence, a line with the step and its time, and a bar. Nothing moves except the seconds. The author asked for what a terminal agent does: the words for what it is doing, with a light crossing them.
2. **The approval is out of sight.** When the agent stops for the person, the buttons that answer it (*Go on*, *Accept both*) were drawn under the waiting step, inside the timeline. The timeline scrolls, and the waiting step is its last row, so in a run of more than a few steps the buttons were below the fold. The author had to scroll to find out that the agent was waiting for a button.

## Change

- While the agent is at work, its sentence has the brand's three strands before it and a band of the agent's colour crossing the words.
- When the agent stops for the person, what it asks is at the top of the card, under the sentence: one line that says what waits, then the buttons. It is not in the timeline.
- What exactly is asked opens under the buttons: the findings, each with its file, line and severity; the plan's files; a failed check's output; the verdict's summary. It is closed until it is opened.

## Decisions

- **The words that move are the agent's own sentence**, the one the engine already writes (*Reading avatars.rs*, *Running checks · bun test*). The author's example was a terminal agent's turning list of words for thinking. Those would be new copy, and they would say less than the sentence does. A list of them is a wording decision for the author and `docs/branding.md`, and can be added on top of this.
- **Motion means heard from.** FEAT-119 took out a pulsing dot because a slow agent and a stuck one looked the same. The author asked for motion again, so it is back, on one condition: it runs only while the run is `starting` or `working` and has not gone quiet (`isWorking`). After three minutes of silence the card already says *Nothing from … for N min*, and the words stop moving then. Waiting, paused, stopped, failed and done are still.
- **The line is not said twice.** The sentence often already says what waits (*Waiting for you: bun test failed*). The line under it is left out when the sentence contains it.
- **The findings line counts by severity** (*2 findings · 1 high, 1 low*), so the line says something the sentence does not.
- **The disclosure is named for what it holds** (*The findings*, *The plan*, *The output*, *The summary*), not *Details*. The card already has a *Details* at the bottom, for what was sent to the agent and what came back, and two buttons with one name in one card would not say which is which.
- **Reduced motion and forced colours** take the band away and leave the words in their own colour. The strands stay, still.

## Acceptance criteria

- A working agent's card has something that moves, and it stops when the agent waits, pauses, ends or goes quiet.
- An agent that waits shows what it asks and the buttons that answer it above the meter and the timeline, with nothing to scroll.
- What exactly is asked is closed by default and opens in place.
- Every button the timeline's approval had is still there and still reaches the engine.

## What was done

- `src/lib/agents/levels.ts`: `isWorking`, `askOf` (the line, and the name of what opens under it, per kind of stop), `planOf`, `findingPlace`.
- `src/lib/agents/AgentCard.svelte`: the sentence and what it asks are one block, `.now`, which is the tinted card while the agent waits. The timeline is steps only.
- `src/app.css`: `.shimmer`, with `shimmer-cross`.
- The same words move on the inbox card's agent line (`src/lib/review/InboxCard.svelte`), and the room's *Agent* tab shows the strands while the agent works (`src/lib/review/ReviewRoom.svelte`), so it can be seen from the Conversation tab.
- Merger's resolve screen draws the same card, so it has both changes.

## Not done

- A turning list of words. See the first decision.
- A mark on the *Agent* tab while the agent waits and the Conversation tab is showing. The card brings itself forward once, on assignment, and a notification says the agent is waiting when those are on.
- Nothing was looked at in the built application. Building it closes the author's running copy.
