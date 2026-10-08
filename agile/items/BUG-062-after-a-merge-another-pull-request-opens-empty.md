<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-062 — After a merge, Pull requests opens another pull request with nothing in it

**Status:** Fixed — on `bugfix/BUG-062-after-a-merge-another-pull-request-opens-empty`.
**Screens:** 1H.
**Raised by:** the author, 2026-10-08, while recording the Review demo against `maxmya/trattoria-demo` with the 1.3.2 release build.

## What happens

1. Open your pull request #4 in the Pull requests workspace and choose *Merge*, then *Create a merge commit*, then *Confirm Merge*.
2. The merge succeeds on GitHub.
3. The workspace now shows a **different** pull request, #2 *Start an allergen table*, with *All Changed Files (0) · No files changed*, *List Of Commits (0) · No commits found*, *No file selected*.
4. It stays like that. Opening #2 from the list shows its 1 file and 1 commit correctly.

There is no word that #4 was merged; the screen simply turns into an empty view of another pull request.

## Cause

`merge()` in `src/lib/requests/store.svelte.ts` reloads the list with `load()` and then `present()`. #4 is no longer open, so `present()` falls back to `openId = next[0]?.id` and calls `clearFiles()`. The view is still the workspace, but nothing calls `loadWorkspaceData()` (or `loadDrafts()`) for the newly selected pull request, as `openWorkspace()` does. The same path probably affects `close()`, which also ends in `load()`.

## Acceptance criteria

- After *Confirm Merge* (and after *Close*), the workspace does not silently turn into another pull request. Either return to the list with the merged one acknowledged, or show the merged pull request as merged.
- If another pull request is opened in the workspace for any reason, its files, commits and drafts are loaded.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/requests/store.svelte.ts` (`merge`, `close`, `load`, `present`, `openWorkspace`, `loadWorkspaceData`) and `src/lib/requests/PRWorkspace.svelte` (`handleMerge`, `handleClose`).
- **Reproduce without a forge:** a store test that seeds two pull requests with the workspace open on the first, then presents a list without it, and checks what the workspace shows.
- **Reproduce for real:** the demo repository `maxmya/trattoria-demo` on GitHub is disposable; open a pull request there and merge it from the workspace.
- **Related:** the success notice for your own merged pull request comes from the notification watcher, a poll later (FEAT-114), not from the merge itself. A notice at merge time may be part of the fix.
- **Branch:** `bugfix/BUG-062-after-a-merge-another-pull-request-opens-empty`, with plan, testing documents and a changelog entry when the work starts.

## Fix

`present()` in `src/lib/requests/store.svelte.ts` now sends the workspace back to the list when the pull request on screen is no longer in the re-read list, which is what a merge or a close leaves behind, instead of opening the first one in its place. If the workspace does land on another pull request, as when it was opened before the list arrived, `present()` reads that one's files, commits and comments; `clearFiles()` already restores its drafts. `PRWorkspace.svelte` says *#N merged* or *#N closed*, with the title, once the host has accepted it.
