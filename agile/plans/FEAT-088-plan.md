<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-088 — Plan

**Item:** [`agile/items/FEAT-088-gitlab-as-its-api-says.md`](../items/FEAT-088-gitlab-as-its-api-says.md)

## Approach

Keep the host-agnostic surface — `PullRequest`, `FileDiff`,
`PullRequestComment`, `DraftComment`, the `review::` functions the Tauri
commands call — and make each GitLab answer underneath it. `review.rs`'s
entry points dispatch on `repo.kind` to `gitlab.rs`, which owns every GitLab
URL and every mapping, each mapping a pure function over JSON with fixture
tests (`file_of`, `commit_of`, `comments_of`, `summarise`, `checks_of`,
`refs_of`, `position`, `line_code`).

Identification: `identify_with(url, accounts)` keeps two segments for GitHub
and Bitbucket and lets GitLab nest, refusing a `-` segment. `Repo::encoded_slug`
percent-encodes everything outside RFC 3986's unreserved set. The Tauri
`forge_repo` passes the connected accounts, so a host an account vouches for is
identified; `forge_connect` asks `identify_account`, which tries GitLab's
`/user` first for a host whose name names no forge.

The inbox's extra facts are a separate command, `review_summaries`, taking
`(repository, number)` pairs so "All my repos" rows are asked of their own
projects; for GitHub it answers nothing without a request. Six workers in a
`thread::scope` ask two endpoints per merge request; a merge request whose
answers fail is left out.

A review is the spec's batch flow. `http::delete` is added so a failure before
`bulk_publish` can take back the draft notes the call made — `bulk_publish`
publishes every draft the person holds, so leaving ours behind would double
them on a retry.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/forge.rs` | `identify_with`, `identify_repo_with`, `encoded_slug`, `encode_segment`, `ReviewSummary`, `review_summaries`, `identify_account`. |
| `crates/spagitty-core/src/forge/gitlab.rs` | Rewritten against the spec. |
| `crates/spagitty-core/src/forge/review.rs` | GitLab dispatch; `DraftComment` ranges and places; GitHub `start_line`. |
| `crates/spagitty-core/src/forge/http.rs` | `delete`. |
| `src-tauri/src/commands.rs`, `lib.rs` | Accounts in `forge_repo`; `review_summaries`; the connect probe. |
| `src/lib/review/store.svelte.ts`, `ReviewInbox.svelte` | Ask for and merge summaries on GitLab. |
| `src/lib/types.ts`, `api.ts` | `ReviewSummary`, `LinePlace`, the draft's new fields. |

## Risks and rollback

- **Two requests per merge request** when the Review inbox opens on GitLab:
  bounded by the list's fifty, six at a time, and only on that screen. A
  self-managed GitLab's default rate limits are far above it.
- **Probing GitLab on connect** sends the token to `/api/v4/user` of the host
  the person typed — the host they are connecting, and nowhere else.
- **Pull requests (1H) on GitLab** places a comment by side alone, as it did;
  on an unchanged line GitLab may refuse it. The review room sends the full
  place.
- Rollback is a revert.
