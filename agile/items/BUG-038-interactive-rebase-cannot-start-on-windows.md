<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-038 — Interactive rebase cannot start on Windows

**Status:** Fixed.
**Branch:** `bugfix/BUG-038-interactive-rebase-cannot-start-on-windows`
**Screens:** Rebase (1E); Conflicts (1D) when continuing a rebase.
**Raised by:** BUG-037. With the fixtures' line endings fixed, two rebase tests
still failed on native Windows, and not because of the fixtures.

## Problem

The Rebase screen hands git the plan through `GIT_SEQUENCE_EDITOR`, pointed at
a small script under the git directory that copies the plan over the todo git
wrote. On Windows the script was a batch file:

```
@echo off
copy /Y "C:\…\.git\spagitty\rebase-todo" %1 >nul
```

git does not run an editor directly. It runs it through a shell, and Git for
Windows through the `sh.exe` it ships, which passes the todo's path in the form
`C:/Users/…/git-rebase-todo`. `copy` read `/Users` as a switch, failed, and git
stopped with:

```
error: there was a problem with the editor '"C:\…\sequence-editor.bat"'
```

No interactive rebase started from Spagitty could run on Windows, whatever the
plan. Continue and Skip use the same scripts for `GIT_EDITOR`.

## Scope

- The sequence and message editors are `sh` scripts on every platform, as they
  already were on Linux and macOS.
- The plan's path inside the sequence editor is written with `/`, which Git for
  Windows' `sh` reads as the same file.

## Non-scope

- A path containing a single quote, which the `sh` script has never quoted for
  on any platform.

## Acceptance criteria

- On Windows, a planned rebase runs the plan: it stops on a conflict with its
  state on disk, and aborting puts the branch back.
- The scripts written on every platform are `sh`, and the plan's path in them
  has no backslash.
