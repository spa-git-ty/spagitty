<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-138 — Preview workflows and follow their runs

**Status:** Backlog — recorded 2026-10-14; not started.
**Branch:** `codex/FEAT-138-pipelines`.
**Screens:** 1Y, chrome.
**Proposal:** [Forge views](../../docs/proposals/forge-views.md). **Designs:** [Forge views](../../design_handoff_forge_views/).

## Change

Draw the working copy’s workflows before a push, then follow runs on watched branches and requests.

## Acceptance criteria

- Parse both workflow formats, local includes, anchors, references, inheritance and matrices as specified in proposal §7.1; keep source lines and report bundled schema problems with nearest-key hints.
- Evaluate the specified triggers and expression subset against event, branch/tag and changed files. Unsupported expressions and remote includes are decided on the host.
- Draw virtual job graphs in Needs and Stages layouts, with edges, expandable matrices, the selected job’s details and file links. Collapse over 200 jobs to stages.
- Read host runs, jobs, steps and logs; persist watched branches and include the default branch and open requests. Poll at the stated intervals only while visible; say when updated.
- Cancel and re-run failed with separate confirmations and useful permission errors. Offer the GitLab lint action explicitly.
- Match both Pipelines artboards; omit local execution as agreed.
- **Tests:** refusal and success cases in core fixture tests, command tests, store tests and mounted screen tests; types, coverage and build pass.

## Not in this item

- Other hosts, workflow editing, local workflow execution and partial-staging stashes are outside 3.1.
- The other three screens have their own items.
