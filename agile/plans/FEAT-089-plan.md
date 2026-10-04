<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-089 — Plan

**Item:** [`agile/items/FEAT-089-a-pull-request-read-from-disk.md`](../items/FEAT-089-a-pull-request-read-from-disk.md)

## Approach

A new core module, `pull.rs`, owns the refs: which host ref holds a head,
where it lands, the refspecs, the fetch (through `shell::fetch_refspecs`, the
one place a process is spawned, `--no-tags --no-write-fetch-head`), resolving
head, base and merge base with `gix`'s `merge_base`, and the worktree. A name
from the webview that could change a refspec's meaning — a colon, a leading
`+` or `-`, whitespace, `..` — is refused before git sees it.

`diff.rs` gains the two-commit forms of what it already does for one commit:
`changes_between` (the file list with counts, `tree_changes` pointed at two
commits) and `full_file_between` (one file with unlimited context). The
existing hunk builder is left alone; `full_lines` walks the same histogram
diff and fills every gap with context. `RawChange` and `FileChange` carry a
rename's old path from gix's `Rewrite`.

The Tauri command that fetches takes what it needs from the session — path,
shared repository handle, the forge remote and its kind — and lets the lock go
before the network: holding the session lock across a fetch would make every
other screen wait on the network. After the
fetch the repository is opened afresh to read the refs it wrote.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/pull.rs` | New. |
| `crates/spagitty-core/src/diff.rs` | `changes_between`, `full_file_between`, `FullFile`, `oldPath`. |
| `crates/spagitty-core/src/shell.rs` | `fetch_refspecs`. |
| `src-tauri/src/commands.rs`, `lib.rs` | `review_checkout`, `review_files`, `review_file`, `review_worktree`. |
| `src/lib/types.ts`, `api.ts` | `PullHead`, `FullFile`, `oldPath`, the four calls. |
| `src/lib/review/store.svelte.ts`, `ReviewInbox.svelte`, `ReviewRoom.svelte` | Open in worktree. |

## Risks and rollback

- **A fetch can ask for credentials.** `GIT_TERMINAL_PROMPT=0` makes it fail
  with git's words instead of waiting; the toast shows them.
- **`refs/spagitty/pull/*` accumulate**, one per pull request reviewed. They
  are small and invisible; pruning them is a later concern.
- Rollback is a revert. The refs and worktrees made stay until removed by
  hand or from the Worktrees dialog.
