<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-138 — Preview workflows and follow their runs

**Status:** Done — on `codex/FEAT-138-pipelines`; unmerged. Covered by fixtures and mounted tests; not inspected in the built application.
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

## What was built

- **Local model:** marked YAML, local includes, references, inheritance, defaults, matrices, bundled schema problems, trigger/condition simulation and dependency depths. The parser caches unchanged file contents. Limits bound nesting, include data and matrix expansion.
- **Host model and commands:** conditional run/job pages, steps, times, scoped caches, pinned-commit configuration graphs, bounded text logs and single-attempt confirmed writes. Credentials stay in Rust. Personal watches are stored per repository/account.
- **Screen:** Preview and Live, Needs/Stages graph, virtual cards/edges, expandable matrices and collapsed stages, selected-job details, source lines, watched branches and requests, duration bars, partial GitLab trace, completed GitHub logs and individual confirmations. Existing chrome stays intact.

## Differences from the spec and the artboard

- **Open source** opens the exact working-copy file and requested line in a read-only panel. Files reads committed revisions, so a link there would show different contents after local edits.
- **Unknown syntax stays on the host.** Conditional, wildcard and remote includes, unsupported glob syntax, expression contexts and dynamic matrices cannot be reported as running or skipped.
- **Default branch context comes from the local remote HEAD.** When unavailable, conditions that require it remain unknown. Live gets its default branch from the host.
- **Live graphs use the run's commit.** Names are matched only when unambiguous; unavailable configuration is explicit and does not prevent reading host job states.
- **Preview bounds:** 1 MiB per file, 8 MiB per workflow's include data, 100 workflow files/includes, 4096 matrix combinations and bounded expression/graph depth. Logs show at most 4 MiB and link to the full job on its host.
- **Local execution is omitted.** GitLab lint sends the current root CI file only after confirmation; GitLab resolves its includes.

## Checks

See the [automated record](../testing/FEAT-138-automated.md) and [manual sweep](../testing/FEAT-138-sweep.md). Added `jsonschema` for offline validation and a direct `regex` reference for bounded GitLab conditions; licences and reasons are recorded there and in `NOTICE`.
