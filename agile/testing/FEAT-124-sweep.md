<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-124 — Sweep

**Item:** [FEAT-124](../items/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top.md)

Rows 01, 03, 04 and 07 were looked at on 2026-10-12 in the development page: the card mounted three times with fixed assignments (working; waiting on two findings after seventeen steps, closed; the same, open), drawn by headless Chrome at 1400 by 820 in the dark and the light scheme. That is the real component and not the real application: no agent ran.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT124-01 | An agent set up for reviews; a pull request open in the room | Assign the agent and watch its card | The sentence has the three strands before it and a band of colour crossing the words, from *Starting* on | P1 | Seen against fixtures, both schemes. Not run in the application. |
| SWEEP-FEAT124-02 | As above | Switch to the Conversation tab while it works; go back to the inbox | The *Agent* tab shows the strands. The inbox card's agent line moves | P2 | Not run. |
| SWEEP-FEAT124-03 | An agent at *Ask every step*, on a pull request of ten files or more | Let it reach a file with findings | The card's top turns into the tinted card: the sentence, *N findings · …*, and *Accept*, *Go to them*, *Go on, decide later*, *Redo with a note*. Nothing has to be scrolled. The words stop moving | P1 | Seen against fixtures with seventeen steps. Not run in the application. |
| SWEEP-FEAT124-04 | As above | Press *The findings* | Each finding opens under the buttons with its file, line, severity and words. The buttons do not move. Press again and it closes | P1 | Seen against fixtures. Not run in the application. |
| SWEEP-FEAT124-05 | An agent at the plan | Press *The plan*, leave a file out, press *Go on* | The files are listed with why each is read; the one left out is not read | P1 | Not run. |
| SWEEP-FEAT124-06 | A merge with checks that fail, an agent assigned in Merger | Let it reach the checks | The card says which command failed with *Go on* and *Run again*; *The output* opens its last lines | P1 | Not run. |
| SWEEP-FEAT124-07 | The desktop set to reduce motion | Watch a working agent | The words are still and fully legible; the strands are still | P2 | Computed style read from the page under the emulated preference: no animation, the text in its own colour. Not run in the application. |
| SWEEP-FEAT124-08 | An agent that says nothing for three minutes | Wait | The words stop moving when the card says it has been quiet | P2 | Not run. |
| SWEEP-FEAT124-09 | A window short enough that the card is under 640 pixels high | Open *The findings* on a file with five or more | The findings scroll in their own box and the card does not overflow | P2 | Not run. |
