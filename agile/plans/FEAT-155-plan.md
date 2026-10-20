<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-155 — Plan

**Item:** [FEAT-155](../items/FEAT-155-untrack.md)

1. `shell.rs`: `worktree_add_unchecked` (`worktree add --no-checkout`, then `read-tree HEAD`), `untrack_paths` (`rm -r --cached`), `staged_names`, `unstaged_names`.
2. `untrack.rs`: `ignore_line` and `with_ignored`; `untrack()` in place on the checked-out branch after the two refusals, or in a scratch worktree removed on drop; the commit through `shell::commit_with` with signing read from the repository and hooks as `hooks::enabled` says (none in the scratch worktree); the remote to push to. `Error::Untrack` for the refusals.
3. An `untrack` command, a writer.
4. Frontend: `untrack` in `api.ts` and its types; `src/lib/untrack/store.svelte.ts`; `UntrackScreen.svelte` on `/untrack`; 2B on the rail with an `untrack` icon; `docs/screens.md`.
