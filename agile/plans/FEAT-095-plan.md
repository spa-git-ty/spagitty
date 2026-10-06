<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-095 — Plan

**Item:** [`agile/items/FEAT-095-check-out-a-pull-request.md`](../items/FEAT-095-check-out-a-pull-request.md)

## Approach

- `pull::check_out(repo, remote, number, head, source, target)` decides the
  name from what the refs say (gix), then writes through `shell`:
  `create_branch` + `set_upstream` for a new branch, `checkout` for one already
  at the head, `switch_reset` (`git switch -C`) for `pr-N` once it is known to
  be an ancestor of the head. The pull request carries no "from a fork" flag,
  so the rule never trusts a name to mean the same branch: only an exact match
  of tips does.
- `review_check_out` (off the main thread, under the session, like
  `checkout`) opens the repository afresh — the fetch before it wrote refs —
  and finds the forge remote as `review_checkout` does.
- `review.checkOut(pr)` replaces `openWorktree`: fetch, check out, say where,
  and refresh the open repository.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/pull.rs` | `check_out`, `CheckedOut`; the worktree code and its tests gone. |
| `crates/spagitty-core/src/shell.rs` | `switch_reset`, `set_upstream`. |
| `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` | `review_check_out` in place of `review_worktree`. |
| `src/lib/api.ts`, `src/lib/types.ts` | `reviewCheckOut`, `CheckedOut`. |
| `src/lib/review/store.svelte.ts`, `ReviewInbox.svelte`, `InboxPreview.svelte`, `ReviewRoom.svelte` | The button and what it does. |
| `src/routes/review/page.test.ts` | The worktree tests become check-out tests. |
| `CHANGELOG.md` | The unreleased worktree entry, replaced. |

## Risks and rollback

- Checking out changes the working copy the rest of Spagitty shows; that is
  the point, and the toolbar's branch picker shows where you are.
- Rollback is a revert, which brings the worktree back.
