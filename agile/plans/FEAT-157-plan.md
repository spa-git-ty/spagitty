<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-157 — Plan

**Item:** [FEAT-157](../items/FEAT-157-close-a-terminal-session-and-give-tmux-our-colours.md)

1. `crates/spagitty-process/src/mux.rs`: `Action::Close` (`kill-session -t <session>`); `Style` and `style_conf`, which writes `set -gq` lines and refuses anything but `#rrggbb`; `Mux::source` (`source-file`).
2. `src-tauri/src/terminal.rs`: `Ask::Close`; a `terminal_style` command that, with the setting on and a multiplexer in use, writes the file to the cache folder, waits up to three seconds for the server, and sources it. Registered in `lib.rs`.
3. `src-tauri/src/settings.rs`: `terminal_colours`, on by default; the frontend's `Settings`, defaults and a Behaviour toggle.
4. Frontend: `retheme()` on the emulator; `multiplexerStyle()` in `xterm.ts`, read from the theme's tokens; the store's `close` ends a session through `close`, `restyle()` and `styleWith()`, colours sent when a session's client starts, and `+` with nothing shown follows the repository; the screen's × on every tab and *Close* in the header behind `dialog.confirm`, and an effect on `theme.revision`.
