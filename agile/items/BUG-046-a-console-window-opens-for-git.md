<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-046 — A console window opens for git

**Status:** Fixed.
**Branch:** `bugfix/BUG-046-a-console-window-opens-for-git`
**Screens:** All, on Windows.
**Raised by:** the author, on Windows: "when i open start review, a cmd prompt
window opens".

## Problem

The release build is a Windows GUI program (`windows_subsystem = "windows"`)
with no console. When such a program starts a console program without
`CREATE_NO_WINDOW`, Windows gives the child a console window of its own. Every
git command in `spagitty_core::shell` was started that way, so each one opened
a window: a flash for a commit or a stage, and a window that stayed for the
length of a fetch. Start review fetches the pull request's head (FEAT-089),
which is long enough to see.

The signing probe (the configured signing program, run with `--version`) and
two of the Farm's spawns — the agent version probe and the reviewer's
`git diff --stat` — had the same gap. The Farm's agent runs and verification
commands already set the flag through `ProcessTree::prepare`.

## Scope

- One constructor, `shell::program`, builds every process `spagitty-core`
  starts, with `CREATE_NO_WINDOW` on Windows.
- The signing probe and the Farm's two spawns use it.
- A test fails if a process is built anywhere else in `spagitty-core`.

## Acceptance criteria

- On Windows, no console window appears for Start review, Fetch, Pull, Push,
  commit, stage or any other git action.
- Elsewhere, nothing changes.
