<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-107 — Hooks you can see and skip

**Status:** Open — on its branch; the manual sweep is not yet run.
**Branch:** `feature/FEAT-107-hooks-you-can-see-and-skip`
**Screens:** 1C, Settings.
**Raised by:** the author, 2026-10-07: switch commit hooks off for a project, skip them for one commit, see what hooks there are and what they run (Husky's code and steps), confirm before they run with OK / Cancel / Skip, and watch their output in a window that matches the design.

## Change

- **Settings → Hooks** (a new section, about the open repository): a switch that turns its hooks off for every commit made in Spagitty, kept as `spagitty.hooks = false` in that clone's `.git/config` (switching back on removes the key); who manages them — git, Husky, lefthook or pre-commit; where git runs them from; and each hook, *On commit* apart from *Other moments*, opening to the script it runs. For Husky that is `.husky/<name>`, not the stub git calls; for lefthook and pre-commit the configuration file with the steps is shown too.
- **Commit bar**: when the repository has commit hooks, a *skip hooks* chip skips them for the next commit only, and *N hooks* opens the same view in a window.
- **Before a commit that runs hooks**: a question naming them — *Cancel*, *Skip hooks*, *Run hooks*.
- **While they run**: a glass window with the hooks' names, a pill that says running, passed or failed, and everything git and the hooks print, line by line as they print it. It closes once git is done; on a failure it stays, with what stopped the commit, and the message is kept.
- **Skipping skips everything**: `--no-verify` alone still runs `prepare-commit-msg` and `post-commit`, so a skipped commit also points `core.hooksPath` at a directory that does not exist.
- The commit no longer holds the repository session while hooks run, so a slow lint does not make other reads wait.

## Acceptance criteria

- Hooks can be read, switched off per repository, skipped per commit, confirmed and watched, on a repository using Husky.
