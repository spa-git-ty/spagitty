<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-130 — Plan

**Item:** [FEAT-130](../items/FEAT-130-a-terminal-on-the-rail.md)

Written on 2026-10-13, before the code.

## The pieces

1. **A PTY** in `crates/spagitty-process/src/pty.rs`, on `portable-pty`: ConPTY on Windows, openpty elsewhere. `spawn` takes a program, its arguments, a directory and a size, and two callbacks: bytes as they are read, and the exit. `write`, `resize` and `kill` are the rest. One thread reads; one waits for the exit. Dropping the handle kills what it started.
2. **The multiplexer's rules** in `crates/spagitty-process/src/mux.rs`, as pure functions first so they are tested without a multiplexer installed:
   - `parse_version` reads `tmux 3.3a`, `tmux next-3.5`, `psmux 3.3.8`.
   - `choose(system, bundled, windows)` is the whole decision: on macOS and Linux, the system tmux when it is at least 3.2, otherwise the bundled one; on Windows, the bundled psmux, otherwise one on the `PATH`; otherwise none.
   - `session_name` makes a session's name from the repository's folder, without the characters tmux keeps for itself.
   - Every command goes to Spagitty's own socket, `-L spagitty`. Attaching is `new-session -A -s <name> -c <path>`. Leaving is killing the client, which detaches and never kills the session.
3. **The commands** in `src-tauri/src/terminal.rs`. A terminal is opened with a Tauri `Channel` that carries its output as raw bytes. Keystrokes and resizes come back as commands. The multiplexer is asked for its sessions and windows, and told to make a window, select one, split and detach. Two more commands check what a link would open, so nothing is underlined on a guess: a hex word must resolve to a commit, and a `path:line` must exist at `HEAD`.
4. **What ships.** `tools/multiplexer.ts` downloads the pinned build for the target and checks its SHA-256 before it is kept: psmux from `psmux/psmux` on Windows, and the tmux project's own static build (`tmux/tmux-builds`) on macOS and Linux. It is staged into `multiplexer/bundle/` and bundled as a Tauri resource, the way the official extension already is, and run from `beforeBuildCommand`. A folder that is not staged is an empty resource, so plain `cargo` and the gates build as before.
5. **The screen** at `/terminal`, code 1T, a rail row after Conflicts.
   - xterm.js with the fit, WebGL and web-links add-ons. WebGL falls back to the default renderer where it is refused.
   - Output is gathered and written once a frame. Scrollback is 10,000 lines.
   - A terminal outlives the screen: its xterm and its element are kept in a registry and put back when the screen is shown again, so leaving and coming back loses nothing.
   - With a multiplexer: one attached client for the repository's session, tabs that are its windows, the Sessions panel, *Split right*, *Split down* and *Detach*. Without one: a plain PTY for each tab, and no panel.
   - *Follow repository* attaches the open repository's session when the repository changes.
   - A link provider finds hex words and `path:line[:col]` on a line, asks which are real, and underlines only those.
6. **Keys.** ``Ctrl+` `` opens Terminal from anywhere and goes back from it. With the terminal focused every key is the shell's except `Ctrl+Shift+P` and ``Ctrl+Shift+` ``; `Ctrl+Shift+C` and `V` copy and paste.
7. **Settings › Behaviour** gains the shell, and whether a multiplexer is used.

## Dependencies

`portable-pty` (MIT) for the PTY. `@xterm/xterm` and its `addon-fit`, `addon-webgl` and `addon-web-links` (MIT) for the emulator. tmux (ISC) and psmux (MIT) as downloaded binaries, never built here.

## Farm sessions

Not in this item: the author answered on 2026-10-13 that they come later. They are recorded as FEAT-135.

## What decides it is right

- A PTY echoes what is written to it, reports a new size, and says when its program exits.
- A system tmux of 3.1 is passed over for the bundled one, and 3.2 is used.
- With no multiplexer there is no Sessions panel and nothing says one is missing.
- A hex word that resolves to nothing is not a link.
- A million lines of output arrive in frames, not in a million writes.
