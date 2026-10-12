<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-076 — Plan

**Item:** [BUG-076](../items/BUG-076-merger-says-pushed-when-the-push-failed.md)

`pushPullRequestHead` called `api.push('', refspec)` itself. Three things
follow from that one call. `push_args` adds the refspec only after a remote, so
an empty remote drops it. The command starts a worker and returns its token, so
the await ends when git starts. And the done event is handled by the network
store, which ignores a token it did not start, so the worker is never released
and the error is never read.

Go through the network store, like every other push:

- `network.whenDone()` is new: it resolves with how the operation running now
  ended, from the same done handler that releases the worker. `fetch` and
  `push` still resolve when the work has started.
- `resolvePullRequestLocally` keeps the remote with the branch and the target.
  It is the prefix of the fetched target (`origin` of `origin/main`), and it is
  kept only when that remote is seen to have the pull request's branch: the
  checkout set an upstream, or `<remote>/<branch>` is among the branches.
- `pushPullRequestHead` refuses with a sentence when there is no such remote or
  when another fetch or push is running; otherwise it pushes, waits, and sets
  *Pushed* or the error from what git said.

Tests replace the network store, not the push command, so they fail if the
store is bypassed again.
