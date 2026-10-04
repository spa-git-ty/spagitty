<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-093 — Plan

**Item:** [`agile/items/FEAT-093-threads-done-properly.md`](../items/FEAT-093-threads-done-properly.md)

## Approach

**Reading.** A new `review_comments`, separate from `pull_request_comments`
so the Pull requests screen reads exactly what it did. On GitHub it is the
REST line comments, matched by `databaseId` to the review threads GraphQL
lists — which gives each its thread's node id and resolved state — plus the
issue comments, with an empty path. On GitLab it is every discussion, the ones
off any line included, each note carrying its discussion's id.
`PullRequestComment` gains `thread_id`.

**Resolving.** `resolve_thread`: a GraphQL mutation on GitHub, `PUT` on the
discussion on GitLab. The room changes the thread at once and puts it back if
the call fails.

**Writing.** `review/drafts.ts` is pure: a line's place by both counters, a
pending comment from a range of a whole file, the draft the backend sends.
`PendingComment` gains the two places and the old path, normalised field by
field like the rest of the record, and a range may now start on the other
side with a larger number. The room keeps the composer's range and the
whole-pull-request text (saved after typing stops), splits pending comments
into those for this head and older ones, and sends Finish review through the
existing `submit_review`, removing what was sent.

**Drawing.** `rows.ts` adds a pending row under each line it covers and the
composer under the range's last line; and, so a card never mixes the
author's lines with a fix's, changes are cut into separate parts when one is
a conflict fix and the next is not.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/forge/review.rs`, `github.rs`, `gitlab.rs` | `thread_id`; `review_comments`, issue comments, threads, `resolve_thread`; GitLab notes off any line and `resolve_discussion`. |
| `src-tauri/src/commands.rs`, `lib.rs` | `review_comments`, `resolve_thread`. |
| `src/lib/types.ts`, `api.ts` | `threadId`; the two calls. |
| `src/lib/review/drafts.ts`, `record.ts`, `threads.ts`, `rows.ts` | Places, pending comments, thread ids, rows. |
| `src/lib/review/room.svelte.ts` | Composer, drafts, body, finish, resolve, reply. |
| `src/lib/review/RoomDiff.svelte`, `RoomConversation.svelte`, `FinishReview.svelte`, `ReviewRoom.svelte` | The writing. |

## Risks and rollback

- **GitHub's thread query** is one more request per open; a token without
  GraphQL access fails the read, which the card reports.
- **A pending comment's lines after a push** may have moved, so it is held
  back rather than placed wrongly.
- Rollback is a revert; the record's new fields are ignored by older code.
