<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-094 — Manual sweep

**Item:** [`agile/items/FEAT-094-review-reads-like-code.md`](../items/FEAT-094-review-reads-like-code.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT094-01 | Release build; a pull request touching Kotlin or TypeScript | 1. Start review | Keywords, strings, numbers, functions, types and comments each in their own colour; changed words still marked | P1 | |
| SWEEP-FEAT094-02 | A file with a block comment over several lines | 1. Whole file | Every line of the comment is in the comment colour | P2 | |
| SWEEP-FEAT094-03 | A Svelte or HTML file | 1. Whole file | Tags, attributes, the script and the style each coloured as what they are | P2 | |
| SWEEP-FEAT094-04 | Review inbox | 1. Drag the preview's left edge | The preview widens; after a restart it is as wide as left | P1 | |
| SWEEP-FEAT094-05 | The review room | 1. Drag the files' right edge and the Conversation card's left edge | Each resizes; double-click resets | P1 | |
| SWEEP-FEAT094-06 | A dependabot pull request | 1. Choose it in the inbox | The table is a table; release notes open from their `<details>` | P1 | |
| SWEEP-FEAT094-07 | A description with a link | 1. Click the link | It opens outside Spagitty; Spagitty stays where it was | P1 | |
| SWEEP-FEAT094-08 | Pull requests screen | 1. Open a pull request's description | Drawn the same way as in Review | P2 | |
| SWEEP-FEAT094-09 | A comment with a code block | 1. Read it in the room | The code is a coloured block, not backticks | P2 | |
