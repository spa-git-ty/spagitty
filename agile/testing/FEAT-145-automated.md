<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-145 — Automated test record

**Item:** [FEAT-145](../items/FEAT-145-history-as-tools-for-agents.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| The protocol | `crates/spagitty-farm/src/history.rs` | It introduces itself with the client's protocol version, lists four tools, answers no notification, and refuses an unknown method. |
| The tools | same | Blame with a line range, a file's history, a commit search and `show` answer from a fixture repository. |
| The cache | same | An answer is cached under its commit; a new commit asks again and finds the new commit; the old commit's answer stays. |
| Refusals | same | A path outside the repository, a missing file and a missing revision are results marked `isError` that say why. |
| Attaching | same | Claude Code gets `--mcp-config` and `--allowedTools` after `-p`, Codex gets two `-c` settings after `exec`, the prompt stays last, other agents are untouched. |
| The prompt | `crates/spagitty-farm/src/context.rs` | The note names the four tools. |

## Not covered

- No live Claude Code or Codex session has called the tools.
- `main.rs`'s flag is not exercised by a test; `serve` is.
