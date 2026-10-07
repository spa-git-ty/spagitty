<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-113 — Visual sweep

**Item:** [FEAT-113](../items/FEAT-113-farm-wrap-up.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT113-01 | Completed farm | Open Farm | Seal pops in once; what landed in landing order with hashes and tries; who did what sorted; Tidy up; Open main in Graph; Start a new goal | P1 | Pass — matches `05-wrap-up.png`. |
| SWEEP-FEAT113-R | The native app, a real repository and installed agents | Run a small farm end to end | Same screens, live | P1 | Not run. Needs the author's machine. |

Run 2026-10-07 against the production bundle in headless Chrome at 1440 × 900, with deterministic IPC fixtures holding the handoff's made-up farm (`.spagitty/farm-qa/`, ignored; screenshots in `screens/`), dark and light. Compared by eye with `design_handoff_farm/screens/`. Reduced motion was checked by reading, not by eye: every Farm animation is CSS, and the shell's `prefers-reduced-motion` rule stops them all. No native Tauri backend, real repository or agent CLI was involved.
