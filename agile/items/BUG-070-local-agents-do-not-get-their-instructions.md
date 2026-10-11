<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-070 — Local agents do not get their instructions, and some cannot start in Review or Merger

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** 1R, 1S, 1K.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## What happens

Command-line agents assigned in Review or Merger do not do what they were asked:

- some act as if they never received the instructions;
- some need a sandbox mode they are not given;
- some refuse because the folder is not trusted;
- agy is not offered at all.

## What is known

On `main`, `crates/spagitty-farm/src/assign/local.rs` launches every local agent the same way: the adapter's command line, plus a read-only flag for Claude Code (`--permission-mode plan`) and Codex (`--sandbox read-only`), with the whole prompt as the last argument.

- **Codex** gets no working directory and no trust or repository check option, so it can fail inside the assignment's scratch worktree.
- **agy** has no adapter.
- **The prompt as an argument (suspected, not verified):** on Windows, npm-installed CLIs are `.cmd` shims, and `cmd.exe` can cut a multi-line argument at the first line break. That would explain *it never got our instructions*.

Pull request #59 (Codex access, OMP and agy launch) addresses Codex, OMP and agy. It is not merged; see TASK-062.

## Acceptance criteria

- Every built-in agent Spagitty detects (Claude Code, Codex, Cursor, OMP, agy) and a custom agent:
  - passes Settings › Agents › *Test*, which proves the agent read a file in the assignment worktree and followed the instruction;
  - completes one Review step and one Merger step on Windows, macOS and Linux.
- The instructions reach the agent whole on every platform. If arguments cannot carry them reliably, they go through standard input or a file the agent is told to read.
- When an agent needs a trust or sandbox choice, Settings says so and offers it, and a refusal says which one was missing.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Order:** finish pull request #59 first (TASK-062), then test every agent against these criteria and fix what is left.
- **Where:** `crates/spagitty-farm/src/assign/local.rs`, `crates/spagitty-farm/src/agent/adapters/`, `crates/spagitty-farm/src/execution/process.rs`, `crates/spagitty-process/src/lib.rs`, `src-tauri/src/agents.rs` (`agents_test_local`).
- **Branch:** `bugfix/BUG-070-local-agents-do-not-get-their-instructions`, with plan, testing documents and a changelog entry when the work starts.
