<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-157 — Automated test record

**Item:** [FEAT-157](../items/FEAT-157-close-a-terminal-session-and-give-tmux-our-colours.md)

- `crates/spagitty-process/src/mux.rs`: only Close ends a session, as `kill-session -t <session>`; the style is the theme's colours as `set -gq` options, with the session's name on the accent and no binding, plugin or unset in it; a colour that is not `#rrggbb`, including one that would end the quoted string, refuses the whole file.
- `src-tauri/src/terminal.rs`: `close` is asked as `Action::Close`; `killSession` and `killServer` are not things the screen can ask.
- `src-tauri/src/settings.rs`: `terminal_colours` is on by default and for a file from before it.
- `src/lib/terminal/store.test.ts`: closing a session ends it, lets go of its client and shows the one beside it; a refusal keeps it and says why; `+` after the last makes the repository's session again; a theme change rethemes every terminal and sends the colours again.
- `src/lib/terminal/xterm.test.ts`: the colours sent are nine `#rrggbb`; an open terminal takes the theme again without failing.
- `src/routes/terminal/page.test.ts`: every tab has a ×; closing from a tab or the header asks, does nothing on no, and ends the session on yes; a plain shell's × asks too; the colours are sent when a client starts.
- `src/lib/settings/sections.test.ts`: Behaviour has six toggles, the colours one on.

By hand, against the bundled psmux 3.3.8 on a throwaway socket (`-L sgtest`, killed afterwards): `set-option -g` takes window options; a `;` chain drops its middle commands; `source-file` takes a whole file, including `set -gq` of an option it does not have; `kill-session -t` ends the session.

## Results — 2026-10-21

- `cargo test -p spagitty-process --lib mux`: 17 passed, 1 ignored (needs a multiplexer).
- `cargo clippy -p spagitty -p spagitty-process --all-targets`: clean. The Tauri crate's own tests run in CI (its test binary does not start on this Windows machine).
- `bunx vitest run`: 229 files, 5081 tests passed.
- `bun run check`: 0 errors, 0 warnings.
