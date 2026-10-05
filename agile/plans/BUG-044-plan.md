<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-044 — Plan

**Item:** [`agile/items/BUG-044-start-review-does-nothing-on-gitlab.md`](../items/BUG-044-start-review-does-nothing-on-gitlab.md)

## Approach

`keyOf` asks `isHere` instead of whether the row names a project: a row of the
open repository takes `requests.repo.host`, any other row the host *All my
repos* searched. The key's owner and name still come from the row's project
when it names one, so a record saved under either scope is the same record.

`ReviewInbox`'s `open` catches and reports through `notice.failed`, as
`openWorktree` already does.

Diagnosed from the author's release build on a self-hosted GitLab: the
remote's path and `references.full` agree, so `isHere` holds for those rows.

## Files

| File | Change |
| --- | --- |
| `src/lib/review/store.svelte.ts` | `keyOf` keys rows of this repository on its host. |
| `src/lib/review/ReviewInbox.svelte` | A failed open is reported. |
| `src/routes/review/page.test.ts` | A GitLab row opens under *This repo*; a failed open is said. |

## Risks and rollback

- A row in *All my repos* that belongs to the open repository is now keyed on
  the open repository's host rather than the searched one. Both are the same
  server: the row matched the open repository by its path. Rollback is a
  revert.
