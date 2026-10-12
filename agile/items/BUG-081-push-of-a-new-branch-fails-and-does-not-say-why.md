<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-081 — Push of a new branch fails and does not say why

**Status:** Open — implemented and tested on `bugfix/BUG-081-push-of-a-new-branch-fails-and-does-not-say-why` for 2.2.3; awaiting merge and the desktop sweep.
**Screens:** chrome, 1A.
**Raised by:** the author, 2026-10-12: Push on a branch that has never been pushed fails with only an exit code instead of publishing the branch or explaining the failure.

## Cause

The toolbar starts a bare `git push`, which refuses a branch with no upstream. The spawned fetch and push inherit stderr instead of piping it to the worker, so neither progress nor Git's error reaches the window. The worker also keeps only the last line: a rejected push's final hint can replace its actual refusal.

## Acceptance criteria

- Push publishes a new branch under its own name on `origin`, or the first remote if there is no `origin`, and sets its upstream without a prompt.
- A branch with an upstream pushes to that remote and branch, including an upstream with a different branch name.
- A repository with no remote explains why no push starts. Metadata read failures are reported.
- Fetch and push stream progress and carry Git's own errors. A rejection followed by hints keeps the rejection reason.
- A rejected push does not change the remote. A successful first push can be followed by another push.

## Implementation

`pushCurrent` uses Merger's existing `pushPlan`. Both spawned Git commands pipe stderr, close stdin and discard unread stdout. The network worker retains `fatal:`, `error:` and rejection lines for failures, with the last nonempty line as a fallback and as the success summary.

Detached HEAD remains a Git refusal. No force push or dependency is added.

## Validation

See [automated checks](../testing/BUG-081-automated.md) and [sweep](../testing/BUG-081-sweep.md). Desktop checks against a real hosting service remain to be run.
