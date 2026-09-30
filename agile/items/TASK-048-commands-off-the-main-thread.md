<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-048 — Commands off the main thread

**Status:** Open.
**Branch:** `task/TASK-048-commands-off-the-main-thread`
**Screens:** none directly — every screen that reads or writes the repository.
**Raised by:** the author, on the revamp: the application "feels slow or
heavy". The review of 2026-09-30 found the likeliest cause in the command layer.

## Problem

A plain `#[tauri::command]` is `ExecutionContext::Blocking`: Tauri runs it on
the main thread, the one that paints the window. BUG-020 found this in the farm
and moved every farm command off it; BUG-012 moved the forge's network calls.
The repository commands were left where they were: 112 of the 126 in
`commands.rs` ran on the main thread, including every one that reads history,
diffs, status, blame, stashes, tags, the reflog, submodules and worktrees, and
opening a repository, which counts its working copy.

On a small repository each takes milliseconds and nothing shows. On a large one
counting status, diffing a big commit or blaming a long file takes long enough
that the window stops painting while it runs — the "heavy" feeling, and on the
worst of them a window that looks frozen.

## Change

- **Every command that touches the repository runs on the async runtime**:
  `#[tauri::command(async)]`, the farm's pattern. They stay ordinary
  synchronous functions — nothing in them awaits — and still take the session
  lock, so two of them never read or write the repository at the same moment.
- **Eleven stay on the main thread**: the ones that write the application's own
  files or the user's git configuration — settings, recents, identity
  profiles, the identity, signing, external tools, forge sign-out, clearing the
  command log. Each is a few milliseconds, and their order is the order the
  user asked in, which a pool of threads would not keep. A test lists them, so
  a new command goes off the main thread unless it is added there on purpose.
- **An open that is overtaken is dropped.** Off the main thread, a large
  repository asked for first can finish reading after a small one asked for
  second. Each open now takes a ticket before it reads, and installs its
  session only while its ticket is still the latest; a close takes one too.
  The overtaken open fails with a new `Superseded` error, and the webview
  ignores the answer to any open or close it has since replaced.

## Non-scope

- `desktop.rs`'s three theme commands: reading a small file and starting or
  stopping a watcher, whose order matters.
- Rewriting commands as `async fn` over `spawn_blocking`. `(async)` blocks a
  runtime worker while a command runs, which is the trade the farm already
  made; a command waiting for the session lock costs a worker, not the window.
- Making any individual operation faster.

## Acceptance criteria

- On a large repository, opening it, switching tabs, scrolling the graph and
  selecting commits do not stop the window painting.
- Of the commands in `commands.rs`, only the eleven named writers are plain
  `#[tauri::command]`, and a test fails if another one is.
- Two opens in quick succession leave the one asked for last open, in the
  session and on screen; an open followed by a close leaves nothing open.
