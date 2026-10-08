<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-064 — Review says "nobody asked you yet" about pull requests you have already reviewed

**Status:** Fixed — on `bugfix/BUG-064-review-says-nobody-asked-you-after-you-answered`.
**Screens:** 1R.
**Raised by:** the author, 2026-10-08, while recording the Review demo against `maxmya/trattoria-demo`.

## What happens

1. You are a requested reviewer on #1 and #2, so both are under *Needs you · you are a requested reviewer*.
2. Send *Request changes* on #1 and *Approve* on #2.
3. Both move to *Open on this repo · nobody asked you yet*.

You were asked, and you answered. The hint is wrong for them, and the inbox no longer tells reviewed pull requests from ones you have never looked at.

## Cause

`src/lib/review/inbox.ts` gives the whole *Open on this repo* group the fixed hint `'nobody asked you yet'`. Once a review is submitted, the host drops you from the requested reviewers, so the pull request falls into that group and takes the hint with it.

## Acceptance criteria

- A pull request you have reviewed, and that has not changed since, is not described as one nobody asked you about. Either its own group or hint (for example *you reviewed it*, with the verdict), or a group hint that is true of everything in it.
- A pull request whose head has moved since your review is told apart from one that has not.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/review/inbox.ts` (the groups and hints), `inbox.test.ts`. Whether the forge layer already returns your latest review per pull request decides how cheap this is; the Pull requests list already shows *approved* and *changes requested*, so the data may be there.
- **Wording:** any new label is product copy, so keep to `docs/branding.md`'s voice.
- **Branch:** `bugfix/BUG-064-review-says-nobody-asked-you-after-you-answered`, with plan, testing documents and a changelog entry when the work starts.

## Fix

The GitHub list now asks for `latestReviews` with each row, and `PullRequest` carries `your_review`: the verdict of the person's own latest submitted review and the head it was left on (pending and dismissed reviews are not counted; GitLab and Bitbucket send none yet). The Review inbox (`src/lib/review/inbox.ts`) gains a group, *Reviewed by you · you left a review*, between *Back with you* and *Open*, so *Open on this repo · nobody asked you yet* only holds pull requests you have not reviewed. Each reviewed card carries *you approved*, *you asked for changes* or *you commented*, followed by *· changed since* in the accent colour when the head has moved past the reviewed commit; those come first in the group.
