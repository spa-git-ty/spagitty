<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-108 — Visual sweep

**Item:** [FEAT-108](../items/FEAT-108-farm-shared-journey.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT108-01 | Any Farm screen | Read the header | Five phases; done ones ticked in `--ok`, the current one ringed in `--accent`, a sub-label under each | P1 | Pass — dark and light, every state below. |
| SWEEP-FEAT108-02 | Board with four agents | Compare badges across board, crew, timeline | One colour and monogram per agent everywhere; no vendor logos | P1 | Pass. |
| SWEEP-FEAT108-R | The native app, a real repository and installed agents | Run a small farm end to end | Same screens, live | P1 | Not run. Needs the author's machine. |

Run 2026-10-07 against the production bundle in headless Chrome at 1440 × 900, with deterministic IPC fixtures holding the handoff's made-up farm (`.spagitty/farm-qa/`, ignored; screenshots in `screens/`), dark and light. Compared by eye with `design_handoff_farm/screens/`. Reduced motion was checked by reading, not by eye: every Farm animation is CSS, and the shell's `prefers-reduced-motion` rule stops them all. No native Tauri backend, real repository or agent CLI was involved.
