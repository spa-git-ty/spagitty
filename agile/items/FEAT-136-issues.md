<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-136 — Issues beside the code

**Status:** Done — on `codex/FEAT-136-issues`; unmerged. Covered by fixtures and mounted tests; not looked at in the running application.
**Branch:** `codex/FEAT-136-issues`.
**Screens:** 1W, chrome.
**Raised by:** the author, 2026-10-14: “focus on the screens !! , apply them all functional end to end”.
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

## What was built

- **Host reads and writes:** `forge/issues.rs` maps GitHub and GitLab lists, comments, events, filters and metadata. `forge/http.rs` adds conditional reads without moving credentials to the frontend.
- **Commands:** `src-tauri/src/issues.rs` resolves configured remotes, caches per account, confirms writes and creates branches through the existing writer. `forge_views.rs` shares this context with the remaining screens.
- **Screen:** `src/lib/issues/` contains typed commands, guarded state, virtual rows, safe Markdown, the thread and details. The shared watcher refreshes subscribed screens while visible.
- **Assignment:** Farm accepts the reviewed issue brief as an unready task draft. Existing execution owns the scratch worktree.

## Differences from the spec and the artboard

- **The group sits above Settings,** following the author's later placement instruction. Existing chrome stays in place.
- **Hand to agent opens the existing Farm task editor.** The 2.0 assignment engine accepts only review and merge jobs. The issue becomes an unready task draft, then uses Farm's agent selection and scratch execution.
- **Remote images stay links,** following the existing Markdown policy.
- **Replies sorting is GitHub only.** GitLab provides no ordering by reply count; typed requests report the limitation.

## Checks

See the [automated record](../testing/FEAT-136-automated.md) and [author sweep](../testing/FEAT-136-sweep.md). No new dependency was added.

## The author's note of 2026-10-14

Later on 2026-10-14, the author said: “اعمل بوش اول ب اول وافتح ال PR” and “عشان اراجع معاك اول ب اول”. Push each checked feature branch and open its pull request for incremental review. This supersedes the earlier no-push decision; no merge remains authorized.

The author approved the proposed implementation choices on 2026-10-14: all listed host writes, each confirmed; existing managers remain in control unless Import is chosen; shared blocks use `.spagitty/hooks.toml`, personal blocks use application data; local pipeline execution is deferred; hidden releases are local; separate stacked branches start at `release/3.0.1`, with no merge or push.

The author then said: “اه كمل كده + في الريل الي عالشمال اعمل فاصل للشاشات الجديده زي البروبوزال ما عامل احنا الردي عندنا فاصل فوق بعد الفارم و فاصل تحت قبل الكونفجرشن حطهم فوق الكونفرجيشن وحط بينهم وبين الي فوقيهم خط فاصل”. The four screens form a separate group immediately above Settings, with a divider above them. Existing chrome is preserved. This later note overrides the proposal’s position after Recipes.
