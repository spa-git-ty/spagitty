<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-141 — Automated test record

**Item:** [FEAT-141](../items/FEAT-141-starting-a-farm-one-step-at-a-time.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| The prompt | `crates/spagitty-farm/src/context.rs` | The planning prompt carries the repository map and no starting points when none were picked; every issue, brief and branch in full when they were; an empty map says so; the map lists top-level files and two levels of folders with counts; a long map is cut and says how many more. |
| The store | `crates/spagitty-farm/src/persistence/store.rs` | The remembered setup reads back as written; a replaced farm is kept, listed newest first and can be taken back once; only the newest twenty are kept. |
| The service | `crates/spagitty-farm/tests/wizard.rs` | A farm with work is kept when a new one starts; one never planned is not; an earlier farm is put back, on disk too, with the current one kept in its place; the crew and rules are remembered with the farm they came from and read by a fresh service; a goal keeps its sources across a restart and names its branch; a farm written before reads as worktrees with no sources. |
| The wizard's state | `src/lib/farm/wizard.test.ts` | Only the goal is required; forward and back stop at the ends; step 3 counts as seen once reached; only Start from has a Skip; one task per issue only with issues; the button's words; exactly what the planner gets, in order; the goal's sources; *Your farm* dimmed and tagged until step 3; the rules in one line; the remembered farm named; resume or start from a past farm; the defaults. |
| The screen | `src/routes/farm/page.test.ts` | Create, configure, remember and plan in that order from Skip to start; Enter goes on from the goal and not while it is empty; Continue and Skip to start need a goal; an empty plan plans nothing; missing agents in one line on step 3; AGENTS.md offered only when there is none; the planner button disabled with no planner; the tracker is Plan, Build, Wrap up; Wrap up's new goal opens the wizard; the Rules sheet's segmented autonomy grants merging at Auto. |

## Not covered

- Not looked at in the running application; the author is checking it.
- The issue picker, the brief picker and the branch picker are not driven against a host or a file dialog.
- Planning a real farm with issues and a brief through a real agent has not been run.
- `cargo test -p spagitty-farm --test pipeline` uses `/bin/sh` agents and runs on Linux and macOS, not natively on Windows.

## Results — 2026-10-18

- `cargo test -p spagitty-farm --test wizard`: 6 passed, 0 failed (Windows).
- `cargo test -p spagitty-farm --lib -- context:: persistence:: model::`: 88 passed, 0 failed (Windows).
- `bunx vitest run src/routes/farm src/lib/farm`: 146 passed, 0 failed.
- `bun run check`: 0 errors, 0 warnings.
