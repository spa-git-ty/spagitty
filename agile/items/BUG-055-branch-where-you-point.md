<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-055 — Branch where you point, stash in place, and a working copy that is noticed

**Status:** Fixed — merged into `main`; the manual sweep is not yet run.
**Branch:** `bugfix/BUG-055-branch-where-you-point`
**Screens:** chrome, 1A, 1R, 1H, 1C.
**Raised by:** the author, 2026-10-07, using the build with FEAT-107: Branch was buggy; Clone and Rebase do not belong in the bottom bar; Stash should stash, not open a screen; Review showed the host's refusal where Pull requests says no account is connected, and both put that state in a corner; and an edit in the working tree was never noticed — the rail said 0 changes with a file changed, and the graph's uncommitted row was gone.

## Change

- **Bottom bar**: Clone and Rebase are gone (Clone stays on the tabs' menu and All repositories; Rebase on the rail).
- **Branch**: the name field opens in the row of the commit selected on the graph, or HEAD's when none is (or another screen is showing). Enter asks *Create branch here* — OK creates the branch there and checks it out; Cancel keeps the field and the name. The question no longer makes the field put itself away when it takes the focus.
- **Stash**: stashes in place. With changes, one question: the message starts as `WIP on <branch>` and can be edited; *Stash now* stashes them, untracked files too. With none, it says *Nothing to stash*.
- **Review → All my repos** with no account for the host says *No account is connected* with Settings → Accounts, as Pull requests does, instead of `github.com refused the token`. Every empty state on both screens sits in the middle of the pane.
- **The working tree is watched.** Only `.git` was, so an edit made in an editor touched nothing Spagitty watched; it looked right only when the editor's own git rewrote the index. The watcher now watches the working tree too, and a burst counts only when one of its paths is not git-ignored, so builds and installs refresh nothing. On Windows, ref changes made outside Spagitty were also missed — the check looked for `/refs/` with a forward slash — and are now recognised by path component.

## Acceptance criteria

- An edit saved in any editor updates the rail count and the graph's uncommitted row within a second.
