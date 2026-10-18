<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-089 — Releases on GitLab is always empty: "responded 302"

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged.
**Screens:** 1Z (Releases)
**Raised by:** the author, 2026-10-18, on 3.1.0: "gitlab.apps.ocp-nonprod-01.hodomain.local: responded 302", then "No releases match this filter." with All 0.

## What happens

On a GitLab repository the release list never appears. The screen shows a 302 error and, under it, an empty filter.

## Cause

- To mark *Latest*, page 1 asked GitLab for `releases/permalink/latest`. That endpoint always answers with a redirect to the release. The forge transport follows no redirect by design (`forge/http.rs`), so the answer was an error, and `mark_latest(..)?` turned that error into a failure of the whole list.
- The first attempt on this branch only hid "No releases match this filter." when an error is shown. That part stays; it was not the cause.

## Acceptance criteria

- GitLab releases list; the newest release that is already out is marked *Latest*, without an extra request.
- On GitHub, failing to read `releases/latest` leaves the list unmarked instead of refusing it.

## Fix

- `forge/releases.rs`: GitLab takes the first listed release that is not `upcoming_release` (`gitlab_latest`); `mark_latest` is GitHub-only and best-effort.
- `ReleasesScreen.svelte`: the empty-filter line is not shown under an error (from the first attempt).
- Test: `gitlab_latest_is_the_newest_release_already_out`.

## Not checked

- Not run against the author's GitLab with a token. Without a token the host answers 404 for the project's API, so the 302 came from the permalink request, as above.
