<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-110 — Visual sweep

**Item:** [FEAT-110](../items/FEAT-110-farm-task-journey.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT110-01 | Working task | Open `/farm?task=T-04` | Stepper on Working with the strands loader; Output tab; Stop and Open the worktree | P1 | Pass — matches `06-task-working.png`, `13-task-light.png`. |
| SWEEP-FEAT110-02 | Ready task | Open `/farm?task=T-02` | "Ready to land. It needs your yes."; filled merge step; Send it back and Land it into main; callout | P1 | Pass — matches `07-task-ready-to-land.png`. |
| SWEEP-FEAT110-03 | Stuck task | Open `/farm?task=T-09` | × on Checks; failing check open; Give up, Edit the task, Retry with Codex | P1 | Pass — matches `08-task-stuck.png`. |
| SWEEP-FEAT110-R | The native app, a real repository and installed agents | Run a small farm end to end | Same screens, live | P1 | Not run. Needs the author's machine. |

Run 2026-10-07 against the production bundle in headless Chrome at 1440 × 900, with deterministic IPC fixtures holding the handoff's made-up farm (`.spagitty/farm-qa/`, ignored; screenshots in `screens/`), dark and light. Compared by eye with `design_handoff_farm/screens/`. Reduced motion was checked by reading, not by eye: every Farm animation is CSS, and the shell's `prefers-reduced-motion` rule stops them all. No native Tauri backend, real repository or agent CLI was involved.
