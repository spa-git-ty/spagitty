<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-064 — Plan

**Item:** [BUG-064](../items/BUG-064-review-says-nobody-asked-you-after-you-answered.md)

Read the person's own review from the host rather than from local state, so a review left on the web counts too. Add `latestReviews(first: 50) { nodes { state author { login } commit { oid } } }` to the GitHub row fragment and map the person's entry to `YourReview { verdict, sha }` (`ReviewVerdict` re-exported from `forge`). Add an optional `yourReview` to the TypeScript `PullRequest`. In `groupInbox`, split the not-requested, no-replies rest into *Reviewed by you* and *Open*; add `pushedSince`, `reviewedLabel` and a `reviewed` chip. Test the mapping in Rust, the grouping and labels in `inbox.test.ts`, and the page.
