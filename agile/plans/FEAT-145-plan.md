<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-145 — Plan

**Item:** [FEAT-145](../items/FEAT-145-history-as-tools-for-agents.md)

Written on 2026-10-19, as the work began.

## The pieces

1. **The tools** — `history.rs`'s `Tools`: blame through `spagitty_core::blame::file`, history and search through `git log`, code search through `git grep`, `show` through `shell::show_text`. Read-only, clipped to 400 lines.
2. **The cache** — a file per question under `.spagitty/farm/history/<commit>/`, named by an FNV hash of the tool and its arguments, with the key written first so a collision is a miss.
3. **The protocol** — newline-delimited JSON-RPC on stdio; `initialize`, `ping`, `tools/list`, `tools/call`; notifications unanswered.
4. **The server** — the application's own executable with `--history-mcp`, handled in `main.rs` before Tauri starts. No new binary to bundle.
5. **Attaching it** — `attach` puts the server into Claude Code's and Codex's command lines; `start_task` does it and adds the *HISTORY* note to the prompt.
