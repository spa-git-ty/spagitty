<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-130 — A terminal on the rail, with tmux in it

**Status:** Done — on `release/3.0.0`, after FEAT-132; unmerged. Covered by Rust tests of a real PTY and of the multiplexer's rules, by mounted tests of the screen with a recording emulator, and looked at in the built frontend in headless Chrome with a pretend shell. On Windows the commands were also run against the psmux that ships, through a PTY. tmux on macOS and Linux was not run, and the screen has not been looked at in the running application.
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

## Second part: its own item

- **Farm sessions** are [FEAT-135](FEAT-135-farm-sessions-in-the-terminal.md). The author answered the spec's question 5 on 2026-10-13: they come later.

## Not in this item

- **A terminal inside other screens** (a drawer).
- **Sessions shared with another machine.**
- **The agent typing into the person's terminal.**

## What was built

- **`crates/spagitty-process/src/pty.rs`**: a PTY on `portable-pty`, with `spawn`, `write`, `resize` and `kill`. One thread reads and one waits for the exit. Dropping the handle ends its program.
- **`crates/spagitty-process/src/mux.rs`**: the multiplexer's rules as pure functions. `parse_version` reads what tmux and psmux print, `choose` decides which binary, `session_name` names a session, and `attach_args` and `action_args` are the only commands sent. All of them go to `-L spagitty`.
- **`src-tauri/src/terminal.rs`**: nine commands. `terminal_open` sends output down a Tauri channel as raw bytes and announces the exit as an event. The terminals are the window's, not the repository's.
- **`files::known`** in the core, behind `terminal_links`: which hex words are commits and which paths are files at `HEAD`.
- **The screen** is `src/routes/terminal/+page.svelte` with `src/lib/terminal/`: the store, `batch.ts`, `keys.ts`, `links.ts`, `xterm.ts` behind the `Emulator` interface, and the Sessions panel.
- **`tools/multiplexer.ts`** downloads the pinned asset for the target and checks its SHA-256: psmux 3.3.8 on Windows, and tmux 3.7c from `tmux/tmux-builds` on macOS and Linux, with the licence texts. It runs from `beforeBuildCommand` and `beforeDevCommand`. The three release workflows set `SPAGITTY_MULTIPLEXER_REQUIRED`.
- **Settings › Behaviour** has the multiplexer's switch and the shell.

## Differences from the spec and the artboard

- **A resource, not a sidecar.** The spec says Tauri `externalBin`. The multiplexer is a folder in the application's resources, the way the official extension is, because a sidecar must exist for every target at every build, and a plain `cargo` build has none.
- **Downloaded, never built.** The tmux project publishes its own static builds, so nothing is compiled in CI.
- **No `THIRD-PARTY-LICENSES.md`.** The criteria name a file this repository does not have. The bundled programs are in `NOTICE`, their licence texts ship beside the binaries, and the linked dependencies are in the generated list Settings shows.
- **Tabs are sessions.** The spec has a tab for each window of a session. A tab here is a session on the socket, and the windows are in the panel, because one client is attached for each session and its windows are tmux's to draw.
- **Splits are tmux's.** *Split right* and *Split down* ask the multiplexer, which draws the panes inside the one terminal. With no multiplexer there are no splits.
- **Windows uses the psmux that shipped** before one on the `PATH`. The system-first rule is for tmux on macOS and Linux, where the author's answer 4 applies.
- **Farm sessions** are not in the panel. See FEAT-135.

## What was found on the way

- **A terminal could fail to start.** On Windows, ConPTY asks where the cursor is before it runs anything, and waits for the answer. The question comes down the channel, which can be ahead of the reply that carries the terminal's id, and an answer with no id to send it to was dropped. What a terminal sends before it has an id is now held, and sent in order once it has one.
