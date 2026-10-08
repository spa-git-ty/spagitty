<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-115 — Automated test record

**Item:** [FEAT-115](../items/FEAT-115-agents-in-review-and-merge.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-farm/src/assign/level.rs` | The levels table is the *Levels and gates* artboard, gate by gate. Unsure stops at every level; failing checks stop even Suggest; anything that always stops turns an unattended landing into a stop; a refused last act goes back to the person and lowers nothing before it. Level changes say which way they went. |
| `crates/spagitty-farm/src/assign/rules.rs` | A repository nobody configured caps at Sign off, allows no verdicts, marks comments and never lands unattended into `main`. Branch patterns match as people write them. |
| `crates/spagitty-farm/src/assign/protocol.rs` | Each answer is read; the last block wins; an unreadable block is no answer; a block inside a stream of JSON events is found; every contract shown to an agent parses. |
| `crates/spagitty-farm/src/assign/classify.rs` | Text is named A, B, both either way, picked lines or Edit; CRLF compares as its text; choices serialise as the resolver's. |
| `crates/spagitty-farm/src/assign/prompt.rs` | The diff keeps its changes and context with both numbers; a finding is on the diff only on a line it shows; every prompt says what is read is data and carries the repository's rules. |
| `crates/spagitty-farm/src/assign/guard.rs` | A clean step touches nothing; what an agent wrote is listed and put back; a change that was there before the step is kept as it was. |
| `crates/spagitty-farm/src/assign/record.rs` | The record reads back as written, atomically; a repository lists newest first; the transcript appends; a broken record is not listed. |
| `crates/spagitty-farm/src/assign/tests.rs` | Against a scripted agent: Step by step stops at the plan, each file and the verdict; Suggest applies nothing; Sign off applies sure findings and waits to send; unsure is never applied; Unattended sends as Comment unless verdicts are allowed; the level is capped; lowering takes effect at once; redo keeps the old step; pause waits for the step; stop cuts a running step; take over says where; a failure says what was kept; an unreadable answer fails with the agent's words; a token limit stops at the end of the step; a moved head waits and *Carry on* marks what follows; *Why?* answers in the timeline; every change is written; a resumed review skips what was done. Merges: Spagitty names the resolution and the checks run on the agent's result; an unattended landing needs configured, passing checks and a branch it may land into; the person's choice is what the checks run. End to end, with a real repository, worktree and process: a CLI agent's write is listed and put back, the worktree is removed and the working copy untouched; a merge's text is named Both. |
| `crates/spagitty-farm/src/assign/local.rs`, `remote.rs`, `world.rs` | Claude Code and Codex run read-only with the prompt last; each step offers the readers and only its own proposer; a tool not offered is refused; what is read counts as sent; a `Co-authored-by` trailer is authorship and prose is not. |
| `crates/spagitty-core/src/models.rs`, `models/http.rs`, `shell.rs` | Each provider is asked and read in its own shape, tool calls and tokens included; a provider's own error sentence is what is said; model lists are read; only this machine is local; plain `http` is refused off it; a key is cut out of an error; `status -z` is parsed. |
| `src-tauri/src/agents.rs` | The machine file starts safe and never holds a key; a short key shows no ending; one job per pull request or merge; a local endpoint needs no consent; a resumed assignment keeps its work and is told how far it got. |
| `src/lib/agents/levels.test.ts`, `AgentsSection.test.ts` | The four levels, their sentences and their cap; the meter, the clock and the card line; where the agent is; the section mounted: detection, key endings, per-job chips, the repository's highest level, verdicts off by default, consent revoked, the first-start offer, Test; the store offers only agents set up, allowed and able, asks once before code leaves the machine, and is empty with nothing set up. |
| `src/routes/review/room.test.ts` | With no agent set up the room says nothing about agents; with one, *Assign an agent…* beside Start review and Check out branch; a finding is drawn where pending comments go, Accept makes it the person's and tells the agent, Dismiss takes it out, Finish review sends only decided ones and marks them as drafted. |
| `src/routes/merge/resolve.test.ts` | With no agent set up nothing mentions agents; with one, the way in beside Resolve 4 conflicts; applied choices carry the agent's badge, an unsure one waits with *Accept*; choosing another way tells the agent and editing the agent's text makes it the person's; the commit says who chose each conflict and credits the agent. |
| `src/lib/palette/commands.test.ts`, `src/lib/farm/store.test.ts`, `src/lib/requests/requests.test.ts` | The agent's controls are commands, greyed with the reason; the farm does not offer an agent switched off for it; requests are made from exactly two files. |

## Results — 2026-10-08

- `bun run test`: 3,656 passed across 177 files.
- `bun run check`: zero errors, zero warnings; the extension SDK check passed.
- `bun run coverage`: passed the configured floors (statements 82.05%, branches 72.24%, functions 79.41%, lines 84.89%).
- `bun run build`: production build passed.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace` on Linux: every suite passed — `spagitty-farm` 410, `spagitty-core` 678, `spagitty` 122, and the rest — including both end-to-end tests with a real worktree and a scripted command-line agent.
- `python3 tools/make-icons.py --check` and `python3 tools/make-brand.py --check`: no drift. `bun audit --audit-level=high`: no vulnerabilities.
- `cargo deny` was not installed in the environment this ran in. The change adds no crate and no package, so gates 1 and 4 have nothing new to judge.
- Not run: the Rust suites natively on Windows and macOS, and any test against an installed agent CLI or a provider's live API.
