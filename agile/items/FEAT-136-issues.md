<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-136 — Issues beside the code

**Status:** Backlog — recorded 2026-10-14; not started.
**Branch:** `codex/FEAT-136-issues`.
**Screens:** 1W, chrome.
**Proposal:** [Forge views](../../docs/proposals/forge-views.md). **Designs:** [Forge views](../../design_handoff_forge_views/).

## Change

GitHub and GitLab issues beside the code, with a branch and an agent one click away.

## Acceptance criteria

- Read the repository’s connected hosts, cache and page 50 issues at a time, drop pull requests from the issue list, and refresh through the existing forge watcher.
- Parse state, assignee, label, milestone and sort filters; show counts, thread, timeline, details and safe markdown.
- Start an editable branch from the default branch through the existing branch operation; hand the full issue to the assignment flow in a scratch worktree.
- Create, comment and close with individual confirmations; hide writes when the credential cannot write.
- Match the Issues artboard’s content structure using shared tokens and virtual rows.
- **Tests:** refusal and success cases in core fixture tests, command tests, store tests and mounted screen tests; types, coverage and build pass.

## Not in this item

- Other hosts, workflow editing, local workflow execution and partial-staging stashes are outside 3.1.
- The other three screens have their own items.
