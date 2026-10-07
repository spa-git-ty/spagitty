<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-109 — Visual sweep

**Item:** [FEAT-109](../items/FEAT-109-farm-building.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT109-01 | Running farm, two working, one checking, one stuck, one ready | Open Farm | "Two agents are working, one task is being checked, and two need you."; Needs you has Land it and Retry with Codex; four columns; crew and Lately | P1 | Pass — matches `04-building.png` and `12-building-light.png`. |
| SWEEP-FEAT109-02 | Paused farm | Open Farm | Sentence and pill say paused; Resume offered | P1 | Pass — matches `04c-building-paused.png`. |
| SWEEP-FEAT109-03 | A run quiet for four minutes | Read its card and crew row | "11 min · quiet for 4 min" in `--warn` | P1 | Pass. |
| SWEEP-FEAT109-R | The native app, a real repository and installed agents | Run a small farm end to end | Same screens, live | P1 | Not run. Needs the author's machine. |

Run 2026-10-07 against the production bundle in headless Chrome at 1440 × 900, with deterministic IPC fixtures holding the handoff's made-up farm (`.spagitty/farm-qa/`, ignored; screenshots in `screens/`), dark and light. Compared by eye with `design_handoff_farm/screens/`. Reduced motion was checked by reading, not by eye: every Farm animation is CSS, and the shell's `prefers-reduced-motion` rule stops them all. No native Tauri backend, real repository or agent CLI was involved.
