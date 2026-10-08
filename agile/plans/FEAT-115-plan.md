<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-115 — Implementation plan

**Item:** [FEAT-115](../items/FEAT-115-agents-in-review-and-merge.md)

The proposal's five slices — Settings › Agents, assignments, review with a local agent, merge with a local agent, remote agents — taken on one branch at the author's request, so 2.0.0 lands as one change. Each slice is its own commit.

## Shape

- **The engine** is `crates/spagitty-farm/src/assign/`: the levels table (`level.rs`), a repository's rules (`rules.rs`), the answer contract (`protocol.rs`), naming a resolution in the resolver's words (`classify.rs`), the prompts (`prompt.rs`), the worktree guard (`guard.rs`), the record (`record.rs`) and the driver loop (`engine.rs`). Spagitty drives the steps and runs the agent once per step through a `Driver`; it asks the levels table what happens at each gate and never asks the agent whether it may go on. A `World` trait is the repository, so every level and limit is tested against a fake one with a scripted agent.
- **Drivers**: `local.rs` runs a command-line agent through the farm's adapters and process runner, adding the provider's read-only mode (Claude Code `--permission-mode plan`, Codex `--sandbox read-only`). `remote.rs` is the loop for a model behind an API, with a small, typed, read-only tool set answered in-process.
- **Model providers** are `crates/spagitty-core/src/models.rs`, the four wire formats, and `models/http.rs`, the second and only other module that reaches the HTTP client. `requests.test.ts` names the two.
- **The Tauri layer** is `src-tauri/src/agents.rs`: the machine list in `agents.json`, keys in the keychain under `agent:<id>`, each repository's rules, *Test*, and the assignments — refused before anything runs when the repository, consent, self-review or one-per-job says so — with changes sent to the webview as events.
- **The webview**: `src/lib/agents/` holds the types, the store, the level words, Settings › Agents, the API agent and custom agent sheets, the Assign popover and the Agent card. `src/lib/review/agent.svelte.ts` keeps the pull request's record in step with the agent — a finding is a pending comment with an author — and `src/lib/merger/agent.svelte.ts` makes resolutions the resolver's choices with who chose them.

## The design's elements, as the kit has them

The handoff was drawn in Claude Design, and some of its controls are not the application's. Each was built from the kit instead:

| In the design | Built as |
| --- | --- |
| Toggle switches (*Agents may approve*, *Mark agent comments*) | A `Chip` reading *on* / *off* beside its label, as Settings › Behaviour does |
| Level picker drawn as toolbar buttons with `role=radio` | A row of `Chip`s, as Settings › Personality chooses |
| Per-job pills (*Review*, *Merge*, *Farm*) | `Chip active`, one group per agent |
| *Claude Code ▾* / *Step by step ▾* default buttons | The themed native `<select>`, as External tools does |
| A modal of its own for *API agent* | The shared `Sheet` through `FarmSheet`, which adds the focus trap |
| The provider's native `<select>` for the model | A text field with a `datalist` of the provider's own list, so a model can be typed |
| The Assign popover and the Agent tab's chrome | The `.floating` popover as Finish review is built, and `Chip` tabs in the card's head |
| The level chip with *▾* on the Agent card | A `Chip` opening the kit's `Menu`, which shows a level above the repository's highest disabled with its reason |
| Icon buttons for Pause and Stop | `Btn` with the icon set's `pause`, `play` and a new `stop` |
| Hexagon, cloud, this-machine, lock, upload marks drawn inline | New entries in `icons.ts`, stroked on the 24-unit grid |
| *Accept* filled in mint | `Btn primary quiet`: one primary per card, in the application's accent |
| The agent's colours as raw hex | `--agent`, `--agent-soft` and `--agent-edge` in `app.css`, light and dark |
| A half-filled dot drawn with a gradient | A ring with a half-width fill, since the interface carries no gradients |

## Order

1. The engine and its tests, against a scripted agent.
2. Model providers and the remote driver.
3. The Tauri layer.
4. Settings › Agents.
5. Review, then Merger, each with its tests and the *no agent, no trace* test.
6. The palette, Farm › Setup, resume, and end-to-end tests with a real worktree and a scripted CLI.
7. The record, the changelog and the 2.0.0 release.
