<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-157 — Close a terminal session, and give tmux Spagitty's colours

**Status:** Done — shipped in 3.9.0, through pull request #88. Covered by Rust, store and screen tests, and the psmux commands were tried against the bundled psmux 3.3.8 on a throwaway socket; not looked at in the running application (the author is checking it).
**Branch:** `feature/FEAT-156-palettes-and-accents`.
**Screens:** 1T (Terminal); Settings › Behaviour.
**Raised by:** the author, 2026-10-21: "in terminal i cant close terminal only detach ! make close button and confirm dialog , match tmux profile colors with spagitty terminal colors + accent ( idk if we want to ignore global tmux pmux configs on the pc )".

## Change

- **Close.** Every tab has a ×, sessions too, and the header has *Close* beside *Detach*. Both ask first — "Close thing? The session ends, with every window in it and what is running there." — and only a yes ends it, with `kill-session`. A plain shell's × asks the same way. After a close the tab beside it is shown; with none left, `+` makes the repository's session again. *Detach* is unchanged, and closing Spagitty still never ends a session.
- **tmux in Spagitty's colours.** The status line, window tabs, pane borders, messages, copy mode and clock take the theme's panel, ink, muted text, line and accent; the session's name sits on the accent, as the tab above it does. They are written as `set -gq` options to a file in the application's cache and loaded with `source-file` into Spagitty's own server after the person's config — when a session's client starts, and again on every theme or accent change. Their key bindings and plugins stay; only colours and the two status lines that carry colours inline (`status-left`, `status-right`) are replaced. A setting in Behaviour, on by default, turns it off.
- **The terminal follows the theme.** An open terminal took its colours once, when it was made; it now takes them again whenever the theme changes.

## Decisions

- The person's own `~/.tmux.conf` or `~/.psmux.conf` is not ignored: it is where their bindings and plugins live, and Spagitty's socket already reads it. Only its colours are overridden, and only on Spagitty's socket.
- The options go in as one sourced file rather than one command each or a `;`-chained command: psmux 3.3.8 silently drops the middle commands of a chain, and takes a sourced file whole, including options it does not know when they are set with `-q`.
- The file carries nothing but `#rrggbb` colours from the webview; anything else is refused before it is written.

## Acceptance criteria

- A session can be closed from its tab and from the header, only after a yes; a no leaves it running.
- Spagitty's terminal and its tmux or psmux bars share one set of colours and the chosen accent, and both change with the theme.
- With the setting off, the colours of the person's own config are left alone.
