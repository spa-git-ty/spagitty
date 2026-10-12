<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-117 — Plan

**Item:** [FEAT-117](../items/FEAT-117-every-merge-happens-in-merger.md)

Give Merger an *origin*: a pull request or a Farm task, set by `presentPullRequest` and `presentFarmTask` and cleared by every other way in. The pull request panel merges through the host; resolving locally reuses the existing plan, resolve and commit flow on the pull request's own branch, then pushes. Farm tasks reuse the plan as it is and tell the Farm after landing. Replace the merge in Pull requests, the Graph and Farm with *Open in Merger*, remove what they leave unused, and add a test that no other file makes a merge.
