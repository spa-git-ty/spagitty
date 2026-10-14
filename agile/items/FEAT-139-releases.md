<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-139 — Draft and publish releases with their files

**Status:** Done — on `codex/FEAT-139-releases`; unmerged. Covered by fixtures and mounted tests; not looked at in the running application.
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

## What was built

- **Host operations:** `forge/releases.rs` maps releases, assets, latest state, confirmed mutations, target/tag resolution, completed artifacts and generated commit notes. Binary transfers stay in `forge/http.rs` and never forward a token through a redirect.
- **Commands and storage:** `src-tauri/src/releases.rs` resolves configured remotes, keeps GitLab drafts and hidden ids locally, commits the changelog through the ordinary path, guards a moved target, and transfers files with progress channels.
- **Screen:** `src/lib/releases/` and `/releases` provide the list, editor, safe preview, file queue, finished artifacts and individual confirmations. The existing chrome is preserved.

## Differences from the spec and the artboard

- **The rail group sits above Settings,** using the forge screen group.
- **Latest and Pre-release are editable on GitHub only.** GitLab has neither field. Its latest badge follows the host's latest-release endpoint.
- **Changelog commit, push and publication are separately confirmed.** A cancelled push retains the committed id so the action can resume without another commit. Existing tags are refused for a new changelog commit.
- **File transfers are bounded to 2 GiB per file.** The file is streamed rather than loaded into memory; failed files can be retried separately.
- **From pipeline queues the downloaded archive before upload.** The person confirms the resulting file upload. The read is limited to the most recent 50 finished runs on the target and pages each run's files.
- **Markdown images remain links,** following the application's existing renderer policy.

## Checks

See the [automated record](../testing/FEAT-139-automated.md) and [manual sweep](../testing/FEAT-139-sweep.md). No new dependency was added.
