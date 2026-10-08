<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-062 — Plan

**Item:** [BUG-062](../items/BUG-062-after-a-merge-another-pull-request-opens-empty.md)

Fix it where the workspace changes hands, in `present()`: when the open pull request has gone from the list and there was one, set `viewMode` to `list`, then pick the first as before. When the open id changes while the view is still the workspace, call `loadWorkspaceData()` after `clearFiles()`. Take the merged or closed pull request's number and title before the call in `handleMerge` and `handleClose`, and post a notice when it succeeds. Add store tests for merge and close with two pull requests, one for a workspace that lands on a pull request when the list arrives, and a component test through *Confirm Merge*.
