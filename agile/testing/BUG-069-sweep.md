<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-069 — Sweep

**Item:** [BUG-069](../items/BUG-069-added-lines-look-red.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG069-01 | A Svelte or HTML file with added lines; light and dark themes | Open it in the review room and read the added lines | Added lines read green, tag names are blue, not red | P1 | Not run. |
| SWEEP-BUG069-02 | As above | Turn on the focus ruler and move it over an added line; select a range of added lines to comment | The line stays green under the ruler and under the comment range | P1 | Not run. |
| SWEEP-BUG069-03 | The Diff screen and a pull request's diff | Read an addition and a removal | Green and red as in the review room | P2 | Not run. |
