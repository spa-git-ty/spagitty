<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-087 — Plan

**Item:** [`agile/items/FEAT-087-the-review-screen-and-inbox.md`](../items/FEAT-087-the-review-screen-and-inbox.md)

## Approach

The inbox is built from the list the Pull requests screen already reads
(`requests` store), so the two screens never disagree about what is open. What
the inbox needs and the list lacked is added to the one row type, `PullRequest`,
as fields that default to nothing — `#[derive(Default)]` on the row and on
`ReviewState`, so a host that cannot answer reads as none rather than a guess,
and the other constructors take `..PullRequest::default()`:

- `head_sha` — `headRefOid` on GitHub, `sha` on GitLab;
- `review_requested` — you are in the review requests (`reviewers` on GitLab);
  apart from `needs_you`, which also covers your own pull request with changes
  requested, which is not a review to do;
- `open_threads`, `resolved_threads`, `replies_to_you` — GitHub's
  `reviewThreads` with the first and last comment's author each, in the same
  GraphQL request (a shared `Row` fragment, so the list and the search ask for
  the same fields);
- `repository` — `owner/name` for a row from a search.

"All my repos" is one GraphQL request holding two searches — `review-requested`
and `involves … -author` — merged by id and sorted newest first (GitLab: the
global `merge_requests` list with `reviewer_username`). Opening such a row asks
the backend which recent repository's remote matches it (`local_clone_of`) and
opens that repository first.

The saved state is the Tauri side's: `review_state.rs` stores a JSON object per
pull request under the app data folder, built from host, owner parts, name and
number, each checked to be a plain name; written by temporary file and rename.
The frontend owns the shape (`review/record.ts`), and `normalise` reads it
field by field so a bad field never costs the pending comments. Outside the
desktop shell the same records go to `localStorage`.

The grouping, sizes, progress and facts are pure functions in
`review/inbox.ts`; the store (`review/store.svelte.ts`) holds scope, selection,
records and the open room; the components draw.

The rail's dot: the layout primes the list once per repository generation,
`review.waiting` reads it, and `NavRail` asks the item rather than only its
count.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/forge.rs` | New row fields; `Default`; `involved_pull_requests`. |
| `crates/spagitty-core/src/forge/github.rs` | `Row` fragment, threads, the two-search query. |
| `crates/spagitty-core/src/forge/gitlab.rs` | `sha`, `reviewers`, project; `involved_merge_requests`. |
| `crates/spagitty-core/src/forge/bitbucket.rs` | Defaults. |
| `src-tauri/src/review_state.rs` | New: the per-pull-request store. |
| `src-tauri/src/commands.rs`, `lib.rs` | `involved_pull_requests`, `local_clone_of`, `review_state`, `set_review_state`. |
| `src/lib/nav.ts`, `ui/icons.ts`, `chrome/NavRail.svelte`, `routes/+layout.svelte` | 1R on the rail, the eye, the dot, the priming. |
| `src/lib/review/*`, `src/routes/review/+page.svelte` | The screen. |
| `src/lib/types.ts`, `api.ts`, fixtures | The new fields and commands. |

## Risks and rollback

- **The list request is larger** by fifty threads per pull request with two
  comment authors each. GitHub counts nodes, not bytes, and the total stays
  three orders of magnitude under its limit.
- **A read on every repository open**, where before the list was read only on
  visiting Pull requests. One request, only with an account connected, and
  it is what makes the dot honest.
- Rollback is a revert; the review files left in app data are inert.
