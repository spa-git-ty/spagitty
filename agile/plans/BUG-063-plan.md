<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-063 — Plan

**Item:** [BUG-063](../items/BUG-063-a-conflicting-pull-request-is-not-marked.md)

Mark `mergeable === false` where the pull request is read, in Review and in Pull requests, from the existing field; leave merging to Merger (FEAT-117), which now offers the way to resolve it. Test the inbox chip and fact, the preview, the room header, the list row and the workspace header, and that an undecided host shows nothing.
