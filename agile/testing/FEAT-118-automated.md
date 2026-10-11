<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-118 — Automated test record

**Item:** [FEAT-118](../items/FEAT-118-an-agents-work-shown-properly.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-farm/src/execution/narrate.rs` | A recorded Claude Code stream becomes typed events (reply, read with its path, command, edit, search, error, result); a Codex-shaped run is typed by what it prints; what Spagitty wrote to the agent and a check's command are typed; a failed tool is an error. |
| `crates/spagitty-farm/src/assign/remote.rs` | A remote agent's calls are typed as reads, tools, results and refusals. |
| `crates/spagitty-farm/src/assign/tests.rs` | An assignment's step keeps what was sent, and the record has the same lines as the events, with the refused finding typed as an error. |
| `src/lib/agents/record.test.ts`, `AgentCard.test.ts` | Adjacent reply lines are one reply; nothing of the prompt or the replies shows until Details is opened; opened, each part is labelled and drawn as itself and a read file reaches `onfile`; Details opens on the step clicked; Raw output is inside Details. |

## Results — 2026-10-11

- `bunx vitest run`: 3,769 passed. `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty-farm` in WSL: all passed (430 in the library, 54 in the pipeline). On Windows the 17 `/bin/sh` tests fail as on `main`.
