<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-145 — History as tools for agents

**Status:** Done — on `release/3.3.0`; unmerged. Covered by Rust tests; not run against a real Claude Code or Codex session.
**Branch:** `release/3.3.0`.
**Screens:** 1Q.
**Proposal:** [A local hub for parallel agents](../../design_handoff_home_farm_notifications/HUB.md), step 4.

## Change

Blame, a path's history, search and a file at a commit, offered to the farm's
agents over MCP and cached per commit, so an agent asks the hub instead of
walking history itself. The agents Spagitty calls by API, which have no folder,
can read files through `show`.

## Acceptance criteria

- **The server** is Spagitty itself: `spagitty --history-mcp <repository>` serves
  MCP on stdio (one JSON-RPC message per line), opens no window, and reads only
  that repository. It answers `initialize`, `ping`, `tools/list` and
  `tools/call`; a notification gets no answer; an unknown method is `-32601`.
- **Four tools, all read-only:**
  - `blame` — path, revision, optional line range: each line with its commit,
    day and author, then the commits' subjects;
  - `history` — the commits that changed a path, following renames;
  - `search` — commit messages, the changes that added or removed a string, or
    the code at a commit;
  - `show` — a file as a commit has it.
  An answer is at most 400 lines and says how many more there were. A question
  it cannot answer (no such commit, a path outside the repository, a binary
  file) is a result marked `isError` that says why.
- **Cached per commit** under `.spagitty/farm/history/<commit>/`, keyed by the
  tool and its arguments. A new commit asks again; an old commit's answer stays.
- **Attached to the agents that take a server:** Claude Code gets
  `--mcp-config` and `--allowedTools mcp__spagitty` (so a headless run may call
  it); Codex gets `mcp_servers.spagitty.*` as `-c` settings. Their prompt gains
  *HISTORY*, naming the tools. Other agents run as before, and their prompt does
  not mention tools they do not have.
- **The farm is told by the application** where the server is
  (`set_history_server`); a farm with none runs agents without it.

## What was built

- `crates/spagitty-farm/src/history.rs`: `serve`, `Tools` (the four tools and the
  cache), `catalogue`, `attach`, `Server`.
- `service.rs`: `set_history_server`; `start_task` attaches it and adds the note.
- `context.rs`: `history_note`.
- `src-tauri/src/main.rs` answers `--history-mcp` before the application starts;
  `farm.rs` gives each farm the server.

## Not checked

- A real Claude Code or Codex run calling the tools: the arguments follow each
  CLI's documented flags, and are covered by unit tests, not by a live session.
- On Windows, a release build is a GUI-subsystem executable; it serves over the
  pipes its parent gives it, which is how an agent CLI starts an MCP server.
