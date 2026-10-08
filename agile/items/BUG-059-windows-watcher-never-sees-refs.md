<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-059 — On Windows the watcher never sees a ref move, and refreshes forever

**Status:** Backlog — reported 2026-10-08, not started.
**Screens:** chrome, 1A, 1S, and every screen that keys off `repo.token`.
**Raised by:** the author, 2026-10-08, while recording the demo videos on Windows 11 with the 1.3.2 release build.

## What happens

- A commit made in Working copy, a merge landed by Merger, or `git commit` in a terminal never shows on the graph. The graph re-walks only when the order is switched (*by date* / *by branch*) or the app is restarted. The status bar's commit count stays stale too.
- While a repository is open and nothing is happening, the backend emits `repo-changed` with `{"refs":false,"worktree":true}` about every 300 ms, indefinitely. Each one runs `repo.refresh()`.

Captured on 2026-10-08 by listening to `repo-changed` from the webview while the app was idle (11 events in 3 s), and while making a commit from a terminal (only `refs:false` events arrived).

## Cause

`src-tauri/src/watch.rs` stores `git_dir = canonical(git_dir)`, and `canonical()` is `Path::canonicalize`. On Windows that returns a verbatim path, `\\?\C:\…\.git`. notify reports events with plain paths, `C:\…\.git\refs\heads\main`. In `classify()`:

1. `path.strip_prefix(git_dir)` fails for every `.git` path, because the prefixes differ.
2. So every `.git` event falls into the working-tree branch and becomes a *candidate*.
3. `.git` paths are not git-ignored, so `any_not_ignored` says yes and the burst is emitted as `worktree: true`.
4. `refs` is never set, so the shell never calls `graph.reload()`.

The refresh that follows touches `.git` again (snapshot, status), which is the next event: the loop the comment on `is_change` warns about, reached by another door. BUG-055 fixed the separator half of this (`/refs/` vs path components); the prefix half remains.

## Acceptance criteria

- On Windows, a commit, merge, branch move or fetch made in Spagitty or outside it re-walks the graph within a second, and the commit count follows.
- An idle open repository emits no `repo-changed` events.
- The working-tree half still works: an edit saved in an editor updates the rail count, and a change to an ignored path does not.
- A unit test drives `classify()` with a verbatim `git_dir` and plain event paths (and the reverse), and both are classified as `.git` paths.

## For the agent who picks this up

**Who:** unassigned. Written so whoever picks it up, an agent or a person, can start cold.

- **Where:** `src-tauri/src/watch.rs`, `canonical()` and `classify()`. Tests live in the same file (`mod tests`).
- **A likely fix:** canonicalise without the verbatim prefix. `dunce::canonicalize` does exactly that, but it is a new dependency, and AGENTS.md asks for the reason to be given in the handoff. Without a dependency: strip a leading `\\?\` (and `\\?\UNC\`) after `canonicalize`, or canonicalise each event path the same way before `strip_prefix`. Mind workdirs too: `workdir` goes through the same `canonical()`.
- **Check on a real Windows machine:** commit in a terminal and watch the graph; leave the app idle and confirm no `repo-changed` traffic. Listening for `repo-changed` from the webview is a quick way to see it.
- **Unblocks:** BUG-060's lost forecast, which is very likely this loop at work.
- **Branch:** `bugfix/BUG-059-windows-watcher-never-sees-refs`. Write the plan and testing documents when the work starts, and a `## [Unreleased]` changelog entry with the fix.
