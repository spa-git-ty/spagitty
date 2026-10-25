<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-099 — Sweep

**Item:** [BUG-099](../items/BUG-099-the-hooks-side-panel-cards-do-nothing.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-BUG099-01 | A repository whose hooks are Husky's or lefthook's | Hooks › Hooks, click an *Add* card | It asks to import; after Import the block is added and selected | Not run; the author is checking it |
| SWEEP-BUG099-02 | A repository with plain git hooks | Hooks › Hooks, click every *Add* card and every block | Each adds or selects | Not run |
| SWEEP-BUG099-03 | Spagitty-run hook records | Hooks › Runs, click each step and each earlier run | The log and the run change | Not run |
