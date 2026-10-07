<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-112 — Visual sweep

**Item:** [FEAT-112](../items/FEAT-112-farm-crew-activity.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT112-01 | Four agents, two running | Open Crew | Tinted now strips, switches in the agent colour, records, Arguments | P1 | Pass — matches `09-crew.png`. |
| SWEEP-FEAT112-02 | Runs and events on record | Open Activity | Timeline fills the pane; striped failed run; dashed review and plan; markers; filtered log | P1 | Pass — matches `10-activity.png`, `14-activity-light.png`. |
| SWEEP-FEAT112-R | The native app, a real repository and installed agents | Run a small farm end to end | Same screens, live | P1 | Not run. Needs the author's machine. |

Run 2026-10-07 against the production bundle in headless Chrome at 1440 × 900, with deterministic IPC fixtures holding the handoff's made-up farm (`.spagitty/farm-qa/`, ignored; screenshots in `screens/`), dark and light. Compared by eye with `design_handoff_farm/screens/`. Reduced motion was checked by reading, not by eye: every Farm animation is CSS, and the shell's `prefers-reduced-motion` rule stops them all. No native Tauri backend, real repository or agent CLI was involved.
