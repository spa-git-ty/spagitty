<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-130 — Automated test record

**Item:** [FEAT-130](../items/FEAT-130-a-terminal-on-the-rail.md)

## Rust

| Where | Cases |
| --- | --- |
| `crates/spagitty-process/src/pty.rs`, against a real shell in a real PTY | **Spawning and writing**: what is written to it is run, and what it prints comes back. **Resizing**: its program is told the size, and the new one. **Exit**: it says when its program exits and with what code; killing it ends its program; dropping it ends its program. It starts in the folder it is told and with the environment it is given. A program that does not exist is an error and not a terminal. A terminal is never made with no room in it. |
| `crates/spagitty-process/src/mux.rs`, the detection rules, with no multiplexer installed | **Versions**: `tmux 3.3a`, `tmux next-3.5`, `tmux openbsd-7.4`, `psmux 3.3.8`, `psmux v3.3.8`, and psmux's two-line answer; what is not a multiplexer has none. **Which binary**: a system tmux is used from 3.2; an older one is passed over for the bundled one; with no system tmux the bundled one is used; with neither there is none, and an old one alone is not used; Windows uses the psmux that shipped and otherwise one on the `PATH`. A file that is not a multiplexer is not one. The names looked for on each platform. **Sessions**: named for the repository's folder, without `.` and `:`. Attaching is `-L spagitty new-session -A -s <name> -c <path>`, with a chosen shell last. Sessions and windows are read from what the multiplexer lists. **No action kills a session.** |
| The same file, one test not run by default | **Live**, against a multiplexer named by `SPAGITTY_MUX`: a client in a PTY makes the session and attaches; a window is made and selected; the session is split both ways; *Detach* ends the client and leaves the session with its windows; a client that is killed, as when Spagitty closes, leaves it too. The session is the test's own and is the only thing removed. |
| `src-tauri/src/terminal.rs` | The shell is the person's own on macOS and Linux, and `/bin/sh` when `$SHELL` is unset or empty. On Windows it is `pwsh`, then Windows PowerShell, then `cmd`, and `$SHELL` is not honoured. With a multiplexer and a session the terminal is its client; a chosen shell is what a new session runs; with no multiplexer or no session it is the shell, plainly; a folder that has gone is not started in. The pill's name, version, path and *bundled*. What the screen asks of a session is what the multiplexer is told, and `killSession` is not something it can ask. |
| `src-tauri/src/settings.rs` | A terminal runs the platform's shell in a multiplexer until told otherwise; a settings file from before the terminal gains both; a shell that is not text costs only the shell. |
| `crates/spagitty-core/src/files.rs` | A hex word is a commit only when it resolves to one, short or full; a word that names a blob is not a commit. A path is a file only when it is one at `HEAD`: a folder and a missing path are not. |

## Frontend

| Suite | Cases |
| --- | --- |
| `src/lib/terminal/batch.test.ts` | A frame's output is written once, in order. A million lines are a write for each frame. A character cut in two is joined before the emulator sees either half. One piece is handed over without a copy. One frame is asked for at a time. An empty piece takes nothing. A flush writes what is waiting, and the frame after writes nothing twice. |
| `src/lib/terminal/keys.test.ts` | Every plain key and every Ctrl key is the shell's. `Ctrl+Shift+P` is the palette and ``Ctrl+Shift+` `` is the way out, found by the key pressed whatever the layout prints. Copy and paste are with Shift on Windows and Linux, and `⌘C` and `⌘V` on macOS, where Ctrl is the shell's. Nothing with Alt is kept. ``Ctrl+` `` (``⌘` `` on macOS) is the key to Terminal and back, and not with Shift or Alt. |
| `src/lib/terminal/links.test.ts` | **Hex words**: seven to forty characters, a word of their own, not part of a name or a path, not a number, not capitals; a word that only looks like a commit is offered to be checked. **Places**: `path:line` and `path:line:col`, where on the line, `./` read off, a root file by its dot, not a label, a clock or an address. **Links**: only what the repository confirmed; a commit by its full id; a word inside a path is the path's; in the order they stand. |
| `src/lib/terminal/store.test.ts` | **With a multiplexer**: the repository's session, named for its folder, in its folder; the multiplexer read once; sessions and the windows of the one shown; a tab for each session and which are detached; another session attached to with the first kept; a window, a selection, a split and a detach asked of the session shown; a refusal said and then forgotten; a session whose client ended is let go and can be attached to again; a session is never closed from here. ***Follow repository***: on until switched off, and kept; another repository's session when it is opened; staying put when off. **With none**: a plain shell named for its program; `+` is another; closing ends its shell and shows the one beside it; a shell that exits takes its tab; the multiplexer is asked nothing. **The channel**: a frame's output written once, a flood in frames, each terminal's keys to its own program, output for a terminal not shown. **Before the id**: what the terminal sends is held and sent in order, and nothing is sent to a tab closed meanwhile. **Size**: told once, again when it changes, and after a slow start. **Failures**: a program that cannot start, a late program whose tab is gone, an unknown exit. **Links**: only what is real, each word asked about once, asked again after a failure and after another repository is opened. **``Ctrl+` ``**: there and back. |
| `src/routes/terminal/page.test.ts` | Mounted, with a recording emulator and a **fake PTY channel**. The session's terminal is in the pane with the focus, and its program is told the pane's size. Tabs, `+`, and no way to close a session. **The pill**: `tmux 3.5a` with the path and version on hover, and *bundled* for psmux. The Sessions panel's sessions and windows. Selecting a window, splitting and detaching. *is detached* and **Attach** after the client ends. Another session from its tab and from the panel. *Follow repository* on, and off. **No multiplexer**: no panel, no pill, no splits, and no text that names one; tabs with a way to close each; **New terminal** when the last is closed. What the program prints is drawn and what is typed is sent. **Leaving and coming back** keeps the terminal and what it printed while away. A commit link opens the Graph and a place in a file opens Files at `HEAD` on its line. |
| `src/lib/terminal/xterm.test.ts` | Without a canvas: an emulator is made with an element of its own, says 80 by 24 and draws nothing until it is in the page, and takes output meanwhile. Scrollback is 10,000. |
| `src/lib/settings/sections.test.ts` | The multiplexer's switch is the fifth, and on. The shell is stored when the field is left or on Enter, trimmed, and not with every letter or when it has not changed. |
| `tools/multiplexer.test.ts` | Every released target has a pinned asset: psmux on Windows and tmux elsewhere, from the project that makes it, at a tag, over https, each file with a whole SHA-256. The binary is taken with its licence, and `NOTICE` names it by name and version. A target Spagitty does not build for is refused. A file with another digest is refused, naming both. What is already staged is not downloaded again, and is for another target, another pin or a missing file. |
| `src/lib/nav.test.ts`, `src/lib/chrome/chrome.test.ts` | Terminal is on the rail after Conflicts with code 1T. |

## Not covered

- **The Rust and the screen together.** Nothing was run in the built application: no key was typed into a real shell through xterm.js.
- **`vim`, `htop` and `less`**, the alternate screen, mouse reporting and wide characters. These are xterm.js's and the PTY's, and neither is tested here beyond bytes in and bytes out.
- **tmux.** The live test was run against psmux on Windows only. The tmux static build was not downloaded or run on macOS or Linux, and neither was the rule that passes over a system tmux older than 3.2 on a machine that has one.
- **The download in CI.** `tools/multiplexer.ts` is tested for what it pins and what it refuses; no release build has run it.
- **``Ctrl+` `` in the layout.** The table that decides is tested; the layout's handler was exercised in headless Chrome only.
- **xterm.js itself**: the WebGL renderer, its fallback, the key handler and the link provider's underline need a canvas, and were looked at in headless Chrome only.

## Results — 2026-10-13

- `bun run test`: 205 files, 4555 tests, all passed.
- `bun run check`: zero errors, zero warnings.
- `cargo test -p spagitty -p spagitty-core -p spagitty-process` (Windows): 137, 765 and 23 passed.
- The live test, `SPAGITTY_MUX=multiplexer/bundle/psmux.exe cargo test -p spagitty-process -- --ignored live` (Windows 11, psmux 3.3.8): passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo fmt --all -- --check`: clean.
- `bun run build`: succeeded.
