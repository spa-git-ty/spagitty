<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-119 — Automated test record

**Item:** [FEAT-119](../items/FEAT-119-an-agents-progress-you-can-read.md)

| Suite | Cases |
| --- | --- |
| `src/lib/agents/levels.test.ts` | Progress counts steps before the plan and files or conflicts after it; the current step and its time; quiet only while working, silent and long enough; the card line for a quiet run and for the step before there are units. |
| `src/lib/agents/AgentCard.test.ts` | The card says the step and its time, counts steps when nothing is known, counts files after the plan, has no pulsing dot, says a run is quiet with a *Stop* that reaches the engine, does not call a short silence quiet, and draws each of starting, working, waiting, paused, stopped, failed and done. |
| `crates/spagitty-farm/src/assign/engine.rs` | An agent that speaks again is no longer quiet and its sentence comes back; hearing an agent that was not quiet changes nothing. |

## Results — 2026-10-11

- `bunx vitest run src/lib/agents`: all passed. `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty-farm --lib quiet`: 3 passed.
