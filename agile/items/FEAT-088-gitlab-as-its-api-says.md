<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-088 — GitLab, as its API says

**Status:** Open — built on `feature/FEAT-088-gitlab-as-its-api-says`, not yet merged.
**Branch:** `feature/FEAT-088-gitlab-as-its-api-says`
**Screens:** 1R, 1H, Settings → Accounts.
**Raised by:** the author, 2026-10-04, while FEAT-087 was being built: "make
sure we comply with those guidance on integration with gitlab (my works server
is gitlab)", with the spec now kept at
`docs/reference/gitlab-merge-requests-api.pdf`.

## Problem

GitLab was read as a list and nothing more, and the list was wrong in four
ways a GitLab user meets on the first day:

- **A project in a nested group was not a project.** `identify` took two path
  segments and refused more, so `team/backend/payments` — the usual shape of a
  company's GitLab — had no forge at all.
- **The project id was half encoded.** Only the slash between owner and name
  was escaped; the spec's `:id` is the whole path URL-encoded.
- **Every merge request needed you, and every one was green.** `needs_you` was
  "not yours"; `checks` was hard-coded passing. A failed pipeline read as
  passing.
- **Everything past the list built a GitHub URL.** Files, commits, comments,
  replies and reviews on a GitLab repository asked `/repos/…/pulls/…` of a
  GitLab host and failed.

And a self-hosted GitLab whose name does not start with `gitlab.` was connected
as GitHub, and never recognised as a forge.

## Change

Every request is one of the spec's endpoints:

- **Projects in nested groups** are identified on GitLab: everything before
  the last path segment is the namespace; a `-` segment (a page about a
  project) is refused. `Repo::encoded_slug` escapes the whole path, and every
  project call uses it.
- **The list says only what it was sent.** Needs you means you are a reviewer;
  checks are *none* in the list; `changes_count` is read as GitLab sends it
  (`"1000+"`).
- **What the list leaves out is asked for** by the Review inbox, a few merge
  requests at a time: the latest pipeline (`…/pipelines`) as checks, and the
  discussions (`…/discussions`) as open and resolved threads and replies to
  you.
- **A merge request's contents**: files from `…/diffs` (and `…/changes` on a
  server older than 15.7, which answers `/diffs` 404), commits from
  `…/commits`, one commit's files from `repository/commits/:sha/diff`, line
  comments from `…/discussions`. Pull requests (1H) works on GitLab with these.
- **Answering**: a reply goes to the discussion holding the note
  (`…/discussions/:id/notes`). A review is the spec's batch: each pending
  comment a draft note with a position pinned to the latest version's base,
  start and head SHAs (`…/versions`), the summary as one more, then
  `…/draft_notes/bulk_publish`; approving is `…/approve` with the head `sha`.
  GitLab has no "request changes" here, so asking for changes publishes the
  comments and withdraws your approval. A failure before publishing deletes the
  drafts it made, so a retry does not publish them twice.
- **Positions as GitLab counts lines**: a draft carries where its line sits —
  added, removed or unchanged, and both versions' counters — so an unchanged
  line is placed by both numbers, and a comment over several lines carries its
  `line_range` by `line_code` (the path's SHA-1 and the counters; SHA-1 from
  `gix`, already a dependency). GitHub reviews carry `start_line` too.
- **Connecting a self-hosted GitLab under any name**: a host whose name names
  no forge is asked GitLab's `/user` first; a connected account's host is then
  identified as its kind.

The token still goes as `Authorization: Bearer`, which the spec allows beside
`PRIVATE-TOKEN`.

## Non-scope

- GitLab's own "request changes" (a newer reviewer-state API), approval rules,
  award emoji, time tracking — the spec's sections 5 to 9 beyond approve.
- Resolving threads and whole-merge-request comments in the review room: the
  threads item.

## Acceptance criteria

- A remote `git@gitlab.example.com:team/backend/payments.git` is a GitLab
  project, addressed as `team%2Fbackend%2Fpayments`.
- A GitLab merge request needs you only when you are a reviewer, and shows no
  checks until its pipeline has been read.
- Pull requests on a GitLab project shows files, commits and comments, and a
  review with comments lands as one published batch.
- A self-hosted GitLab not named `gitlab.` connects as GitLab.
