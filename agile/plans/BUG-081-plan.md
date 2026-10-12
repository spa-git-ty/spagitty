<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-081 — Plan

**Item:** [BUG-081](../items/BUG-081-push-of-a-new-branch-fails-and-does-not-say-why.md)

1. Reuse `pushPlan` to name the current branch's destination, catching metadata failures and refusing a missing remote before starting the worker.
2. Pipe fetch and push stderr into their worker. Discard stdout because it has no reader and close stdin because the desktop has no terminal prompt.
3. Keep Git's refusal lines separately from the last progress or hint line. Preserve existing progress events and successful summaries.
4. Test destination selection, a real first and second push to a temporary remote, streamed fetch progress, missing-remote errors, and a rejected push's done event and unchanged remote.
5. Run the frontend suite and checks, Rust tests, formatting and clippy, and build the application before opening the 2.2.3 pull request.
