<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-130 — Sweep

**Item:** [FEAT-130](../items/FEAT-130-a-terminal-on-the-rail.md)

The rows marked *headless* were run on 2026-10-13 against the built frontend in headless Chrome at 1440×900, dark and light, with the real xterm.js and a pretend shell and pretend sessions behind it. That shows the screen and the emulator. It does not show a PTY or a multiplexer. The rows marked *live* were run the same day on Windows 11 against the psmux that ships, through a real PTY, by a Rust test. None of these rows has been run in the running application, and none on macOS or Linux.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT130-01 | A repository open | Press **Terminal** on the rail | It is in Repository, after Conflicts. A terminal opens in the repository's folder, with the focus | P1 | The rail row and the screen passed headless. Not run in the application. |
| SWEEP-FEAT130-02 | Any screen | Press `Ctrl+`` (`⌘`` on macOS), then again | Terminal, and then the screen you were on | P1 | Passed headless from the Graph. The way back was not pressed. Not run in the application. |
| SWEEP-FEAT130-03 | Settings › Behaviour, shell empty | Open a terminal on each platform | `$SHELL` on macOS and Linux; `pwsh`, else Windows PowerShell, else `cmd` on Windows | P1 | Not run. Covered by Rust tests. |
| SWEEP-FEAT130-04 | Settings › Behaviour | Type a shell in **Terminal shell**, leave the field, open a new session | The new session runs that shell | P2 | Not run. The field is covered by a mounted test and the argument by a Rust test. |
| SWEEP-FEAT130-05 | A terminal open | Run `vim`, `htop` and `less` on a long file; resize the window in each | Each draws, takes keys and the mouse, redraws at the new size, and gives the screen back on leaving | P1 | Not run. Needs the application. |
| SWEEP-FEAT130-06 | A terminal open | Print colours, wide characters and an emoji | They are drawn as a native terminal draws them, in the theme's colours | P2 | Colours passed headless in dark and light. Wide characters not run. |
| SWEEP-FEAT130-07 | Windows, a build with the bundle staged | Open Terminal | The pill says `psmux 3.3.8 bundled`, and its hover gives the path in the application's resources | P1 | The pill passed headless with a made-up answer. The staged psmux answered `-V` and was run live. Not run in the application. |
| SWEEP-FEAT130-08 | macOS or Linux with tmux 3.2 or later | Open Terminal | The pill names the system tmux, without *bundled* | P1 | Not run. Covered by a Rust test of the rule. |
| SWEEP-FEAT130-09 | macOS or Linux with tmux older than 3.2, or none | Open Terminal | The pill says `tmux 3.7c bundled` | P1 | Not run. Covered by a Rust test of the rule. The static build was never run. |
| SWEEP-FEAT130-10 | A multiplexer in use | Run `tmux -L spagitty list-sessions` in another terminal; then `tmux list-sessions` | The repository's session is on Spagitty's socket, and the person's own server lists nothing of Spagitty's | P1 | Passed live: a session made on one socket was not listed on the default one. |
| SWEEP-FEAT130-11 | A session with a program running in it | Close Spagitty; open it again and go to Terminal | The program is still running and its output is there | P1 | Passed live for the session: it outlives a client that was killed. Not run by closing the application. |
| SWEEP-FEAT130-12 | A session shown | Press **Detach**, then **Attach** | *is detached*, the session still listed with its windows; then the terminal again, as it was | P1 | Passed headless, and live: detaching ended the client and left the session and its two windows. |
| SWEEP-FEAT130-13 | A session shown | Press `+`, a window in the panel, **Split right** and **Split down** | A new window; the chosen window shown; panes side by side and one above the other | P1 | The panel passed headless. The commands passed live. The panes were not looked at. |
| SWEEP-FEAT130-14 | Two repositories open, *Follow repository* on | Switch repository tabs | The terminal switches to that repository's session, making it the first time | P1 | Not run. Covered by store and mounted tests. |
| SWEEP-FEAT130-15 | As above | Switch *Follow repository* off and switch tabs | The terminal stays where it is | P2 | Not run. Covered by store and mounted tests. |
| SWEEP-FEAT130-16 | A `~/.tmux.conf` that changes the prefix or the status line | Open Terminal | The change applies in Spagitty's sessions | P2 | Not run. |
| SWEEP-FEAT130-17 | Settings › Behaviour, the multiplexer switched off, or a machine with none | Open Terminal | A plain terminal for each tab, with a way to close each. No Sessions panel, no pill, and nothing that says one is missing | P1 | Passed headless. Not run in the application. |
| SWEEP-FEAT130-18 | A repository with history | Run `git log --oneline`, and hover the ids and a word such as `decade1` | Each real id is underlined and opens on the Graph. The word is not underlined | P1 | Passed headless: three ids and the word were each asked about once, and only the ids were links. Not run against a repository. |
| SWEEP-FEAT130-19 | A build that fails | Hover a `path:line:col` in the compiler's output, and one for a file that is not at HEAD | The real one is underlined and opens in Files on that line. The other is not | P1 | Passed headless for the underline. Files was not opened. |
| SWEEP-FEAT130-20 | The focus in the terminal | Press `Ctrl+F`, `Ctrl+K`, `Ctrl+C`; then `Ctrl+Shift+P`; then `Ctrl+Shift+`` | The first three go to the shell and nothing in Spagitty answers. Then the palette. Then the screen you came from | P1 | `Ctrl+F` and the palette passed headless. The rest not run. |
| SWEEP-FEAT130-21 | Text selected in the terminal | Press `Ctrl+Shift+C`, then `Ctrl+Shift+V` (`⌘C`, `⌘V` on macOS) | The selection is copied, and pasted at the prompt | P2 | Not run. |
| SWEEP-FEAT130-22 | A terminal open | Run `yes \| head -n 1000000`, and use the rail while it runs | The window does not freeze, the rail answers, and the last lines are there at the end | P1 | Passed headless with a million lines from the pretend shell: no frame over 50 ms, the worst 33 ms dark and 50 ms light. Not run through a PTY. |
| SWEEP-FEAT130-23 | A terminal with output | Go to another screen and come back | Everything printed is there, with anything printed while away | P1 | Passed headless. |
| SWEEP-FEAT130-24 | A terminal with more than 10,000 lines printed | Scroll to the top | The oldest line is 10,000 above the screen | P3 | Not run. |
| SWEEP-FEAT130-25 | A release build on each platform | Look in the application's resources, and at `NOTICE` | `multiplexer/` holds the binary for the platform and its licence texts, and `NOTICE` names both programs | P1 | Staged on Windows by `tools/multiplexer.ts` with the digest checked. No release build was made. |
| SWEEP-FEAT130-26 | A build with no network | Run a plain `cargo build`, then a release build | The first builds with an empty resource. The second fails and says the multiplexer could not be fetched | P2 | Not run. |
| SWEEP-FEAT130-27 | Light theme | Open Terminal and run a command with colours | Text, colours and the cursor read as clearly as in dark | P2 | Passed headless. |
