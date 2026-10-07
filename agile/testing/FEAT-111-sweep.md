<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-111 — Visual sweep

**Item:** [FEAT-111](../items/FEAT-111-farm-plan-journey.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT111-01 | No farm | Open Farm | Headline, six-chip flow, numbered crew, goal and rules cards, glass bar | P1 | Pass — matches `01-setup.png`, `11-setup-light.png`. |
| SWEEP-FEAT111-02 | Planner running | Open Farm | Orb, "Claude Code is planning", streamed lines with kinds, older lines dimmed, Stop planning | P1 | Pass — matches `02-planning.png`. |
| SWEEP-FEAT111-03 | Nine drafts | Open Farm | Three waves; pill with counts; Who does what, When it starts, Worth a look | P1 | Pass — matches `03-plan-review.png`. |
| SWEEP-FEAT111-R | The native app, a real repository and installed agents | Run a small farm end to end | Same screens, live | P1 | Not run. Needs the author's machine. |

Run 2026-10-07 against the production bundle in headless Chrome at 1440 × 900, with deterministic IPC fixtures holding the handoff's made-up farm (`.spagitty/farm-qa/`, ignored; screenshots in `screens/`), dark and light. Compared by eye with `design_handoff_farm/screens/`. Reduced motion was checked by reading, not by eye: every Farm animation is CSS, and the shell's `prefers-reduced-motion` rule stops them all. No native Tauri backend, real repository or agent CLI was involved.
