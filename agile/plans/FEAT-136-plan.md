<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-136 — Plan

**Item:** [FEAT-136](../items/FEAT-136-issues.md)

Written on 2026-10-14, before the code.

## The pieces

1. **Host-neutral issues** in `forge/issues.rs`: typed issues, comments, events, labels and milestones; filter parsing and host query mapping; paged lists and threads; confirmed create/comment/close requests. Pure mappings and injected transport tests cover both hosts without credentials.
2. **Conditional reads** in `forge/http.rs`: add a separate response carrying ETag, pagination and token scope headers. Existing callers keep their response type and HTTPS/redirect rules.
3. **Commands and local cache** in `src-tauri/src/issues.rs`, with shared context helpers in `forge_views.rs`: identify actual remotes using connected accounts, select one by remote name, keep credentials in Rust, paint cached pages before revalidation, and report missing accounts separately from empty lists.
4. **Local actions** use the existing branch writer and assignment machinery. Branch names are editable and start from the host's default branch. The issue title, body and comments form the assignment brief; execution uses a scratch worktree.
5. **Typed state and the screen** in `src/lib/issues/` and `/issues`: virtual issue rows, structured and text filters, thread and details, separate errors, pagination, per-action confirmation and repository-generation guards. Reuse safe Markdown and the existing assignment controls.
6. **Refresh and navigation**: subscribe to the existing forge watch cycle, add Issues before Hooks in the new group, expose it in the palette and Rail settings, and update the item, test record, sweep, screens and changelog.

## What is decided, and why

- **Remote names are the selector.** Commands resolve them from the open repository. The frontend cannot supply an arbitrary authenticated host URL.
- **A cached answer is provisional.** It paints immediately; a failed refresh retains it and says why. Cache keys include repository, account, filter and page.
- **Writes are one attempt.** Transport failures may follow delivery, so no automatic retry creates duplicate issues or comments.
- **Known missing permissions hide writes.** Scope headers and host refusals are authoritative. An unknown scope is not presented as proof of access; the host still checks every write.
- **Incremental review.** This branch starts at Hooks and its pull request targets the Hooks branch, as the author asked.

## Where this differs from the item or the proposal

- **Navigation placement** follows the author's later instruction: a separate group above Settings.
- **Existing Markdown image policy** remains authoritative: images are safe links, rather than loading arbitrary remote images into the webview.
- **Issue assignment uses the existing Farm task editor.** The 2.0 assignment engine accepts review and merge jobs only. The issue brief opens as an unready Farm draft for review and agent selection; its existing executor owns the scratch worktree. No agent starts from the issue button.
- **Replies sorting is offered on GitHub only.** GitLab's issue API has no replies ordering; typed requests for that ordering report the limitation.

## Dependencies

None. Existing HTTP, JSON, Markdown and assignment infrastructure are reused.

## Not in this plan

- Bitbucket issue operations and the unnamed future host.
- The remaining Releases and Pipelines items.

## What decides it is right

- Host fixture tests discard pull requests, map GitLab system notes as events, preserve pagination and handle 304 responses.
- Query tests cover quoted labels, state, assignee, milestone, sorting and free text.
- Write tests require confirmation, validate input and never retry an uncertain delivery.
- Store tests keep cached pages and independent errors, reject stale replies and hide writes after a permission refusal. Mounted tests exercise selection, filters, creation, comments and closing.
- Branch and assignment fixture tests keep the person's working copy safe. Types, full tests, configured coverage and build pass before the commit and pull request.
