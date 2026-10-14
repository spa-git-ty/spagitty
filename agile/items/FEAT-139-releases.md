<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-139 — Draft and publish releases with their files

**Status:** Backlog — recorded 2026-10-14; not started.
**Branch:** `codex/FEAT-139-releases`.
**Screens:** 1Z, chrome.
**Proposal:** [Forge views](../../docs/proposals/forge-views.md). **Designs:** [Forge views](../../design_handoff_forge_views/).

## Change

Read, draft, publish and manage releases and their attached files beside the repository.

## Acceptance criteria

- Read GitHub and GitLab releases and files. Show filters, states, downloads only where available, and locally hidden releases.
- Edit tag, target, since, title, pre-release, latest, body and changelog options; preview safe markdown and generate notes from the commit range using the shared message parser.
- Keep GitLab drafts locally; create, save, publish and delete with individual confirmations. Delete tags only when separately selected.
- Attach and remove files, copy links and import finished pipeline artifacts with per-file progress and retry.
- Write CHANGELOG.md through the ordinary commit path before tagging; run hooks and stop if the remote target moved.
- Match the Releases artboard’s list, editor and files column.
- **Tests:** refusal and success cases in core fixture tests, command tests, store tests and mounted screen tests; types, coverage and build pass.

## Not in this item

- Other hosts, workflow editing, local workflow execution and partial-staging stashes are outside 3.1.
- The other three screens have their own items.
