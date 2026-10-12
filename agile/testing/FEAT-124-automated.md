<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-124 — Automated test record

**Item:** [FEAT-124](../items/FEAT-124-an-agent-that-shows-it-is-working-and-asks-at-the-top.md)

| Suite | Cases |
| --- | --- |
| `src/lib/agents/levels.test.ts` | At work only while starting or working and heard from. Nothing is asked unless the run waits. The findings line counts by severity, one and many. Nothing more to open when nothing is left to decide. The plan's line and its files; a plan with no files. Which check failed, and checks that passed. The last act, to send and to merge. A conflict's why, or how it was resolved when there is no why. The verdict as its step says it. A stop between steps. Where a finding is. |
| `src/lib/agents/AgentCard.test.ts` | The sentence moves while the agent works, with the strands before it. It is still when the run waits, is paused, stopped, failed or done, and once it has gone quiet. What is asked is in the block at the top, before the meter and the timeline, and the timeline has no buttons of its own. Nothing is asked when the agent is not waiting. The findings are closed, open with file, line, severity and words, and close again. The line is left out when the sentence already says it. The verdict's summary opens. The earlier cases for each kind of stop still pass against the new place: the plan's files are dropped after *The plan* is opened, and a failed check's output is read after *The output* is. |

## Not covered

- That a question opened on one step is closed when the agent stops on the next. It follows from keeping the open step's index, and the mount helper cannot change a property after mounting.
- The inbox card's line and the room's *Agent* tab. Both use `isWorking`, which is covered; the markup is not.
- The motion itself. `flat.test.ts` and `glass.test.ts` read `app.css` and pass with the new rule in it.

## Results — 2026-10-12

- `bunx vitest run`: 189 files, 4066 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `bun run build`: succeeded.
- No Rust changed, so `cargo test` was not run.
