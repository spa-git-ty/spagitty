<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-084 — Manual sweep

**Item:** [`agile/items/FEAT-084-the-other-screens-in-the-spatial-language.md`](../items/FEAT-084-the-other-screens-in-the-spatial-language.md)

Run 2026-09-30 in the Windows 11 release build, 125% display scaling, light
theme, driven over the webview's debugging port: screenshots before and after
on the same repository, and the pointer moved by the protocol rather than by
hand.

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT084-01 | A repository with many branches | 1. Branches | No row rules, no column lines; buttons only on the hovered row; rows the lists' size | P1 | Pass, 2026-09-30 |
| SWEEP-FEAT084-02 | Tags, one with a long name | 1. Tags 2. Hover a row | The long name ends in an ellipsis, the commit id beside it is whole and clear of the message; buttons on the hovered row only | P1 | Pass, 2026-09-30 |
| SWEEP-FEAT084-03 | A long reflog | 1. Reflog 2. Hover a row | As -01; the refs chips have no rule under them | P1 | Pass, 2026-09-30 |
| SWEEP-FEAT084-04 | Any repository | 1. Every screen | No hairline under a header or over a footer inside the pane | P1 | Pass, 2026-09-30 |
| SWEEP-FEAT084-05 | No farm yet | 1. Farm | The title is the size of every other screen's; one card, the goal; steps and checks as lines | P2 | Pass, 2026-09-30 |
| SWEEP-FEAT084-06 | — | 1. File history with no file | The prompt is centred in the pane, a glass card | P2 | Pass, 2026-09-30 |
| SWEEP-FEAT084-07 | — | 1. Settings | Identity Profiles' heading is the size of You, Signing and Accounts | P2 | Pass, 2026-09-30 |
| SWEEP-FEAT084-08 | Keyboard only | 1. Branches 2. Tab into a row | Its buttons appear with the focus | P2 | |
| SWEEP-FEAT084-09 | Dark theme | 1. As -01 to -04 | The same | P2 | |
| SWEEP-FEAT084-10 | Linux | 1. As -01 to -04 | The same | P3 | |
