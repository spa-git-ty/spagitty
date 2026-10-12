<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-130 — A terminal on the rail, with tmux in it

**Status:** Backlog — recorded 2026-10-12 for 3.0.0; not started.
**Branch:** `feature/FEAT-130-a-terminal-on-the-rail`.
**Screens:** 1T (Terminal, new, at `/terminal`), chrome (a rail row), 1K (Settings › Behaviour: shell and multiplexer).
**Raised by:** the author, 2026-10-12: a full terminal in the left bar so nobody leaves Spagitty for one, supporting tmux and pmux, shipped in where the licence allows.
**Proposal:** [`docs/proposals/workbench-and-recipes.md`](../../docs/proposals/workbench-and-recipes.md) §5. **Designs:** [`design_handoff_workbench/`](../../design_handoff_workbench/) — `Main.dc.html`.

## Change

Terminal is a real terminal on the rail:

- **The pieces**: a PTY per tab from `crates/spagitty-process` (`portable-pty`), drawn by xterm.js with the WebGL renderer. It runs inside a multiplexer on Spagitty's own socket, with one session per repository, and it follows the open repository.
- **What ships**: psmux on Windows, and a static tmux on macOS and Linux where the system has none at 3.2 or later. Both are sidecars, with their licences in `NOTICE`.

## Acceptance criteria

- **A rail row** in Repository, after Conflicts, with code 1T. `Ctrl+``/`Cmd+`` opens Terminal from anywhere and goes back from it.
- **The shell**:
  - A tab runs the shell from Settings: `$SHELL` by default on macOS and Linux; `pwsh`, then Windows PowerShell, then `cmd` on Windows.
  - Resize, colours, the alternate screen, mouse reporting and Unicode behave as in a native terminal. `vim`, `htop` and `less` work.
- **The multiplexer**:
  - Spagitty's sessions live on `tmux -L spagitty` (psmux on Windows), one per repository, created with `new-session -A -s <name> -c <path>`.
  - The person's own tmux server is never touched, and their `~/.tmux.conf` applies.
  - Closing Spagitty detaches and never kills.
- **Which binary**: the system tmux is used when it is at least 3.2; otherwise the bundled one. Bundled binaries are built or downloaded in CI at pinned versions with checksums, and each licence (ISC, libevent BSD-3-Clause, ncurses, utf8proc MIT, psmux MIT) is in `NOTICE` and `THIRD-PARTY-LICENSES.md`.
- **No multiplexer**: when none can run, each tab is a plain PTY, the Sessions panel is hidden, and nothing says anything is missing.
- **The header pill** names the multiplexer and *bundled* or not. Its hover title gives the path and version.
- ***Follow repository*** (on by default) switches the terminal to the repository tab's session when the tab changes, and creates the session if needed.
- **Links**:
  - A hex word of 7 to 40 characters is underlined only if it resolves in this repository, and opens it on the Graph.
  - A `path:line` is underlined only if it exists at HEAD, and opens it in Files.
- **Keys**: while the terminal has focus, every key goes to the shell except `Ctrl+Shift+P` (palette) and `Ctrl+Shift+`` (leave).
- **Performance**: output is batched per frame; scrollback is 10,000 lines by default. `yes | head -n 1000000` does not freeze the window, and the rest of the app stays responsive.
- **Tests**: Rust tests cover spawning, writing, resizing and exit of a PTY, plus the multiplexer detection rules. Component tests cover the screen with a fake PTY channel.

## Second part (may be its own item)

- **Farm sessions**: the farm may run a local agent inside `tmux -L spagitty` as `farm/<task>`. Such sessions are listed with the agent's mark and state, and attach read-only. This needs the author's answer to the spec's question 5.

## Not in this item

- **A terminal inside other screens** (a drawer).
- **Sessions shared with another machine.**
- **The agent typing into the person's terminal.**
