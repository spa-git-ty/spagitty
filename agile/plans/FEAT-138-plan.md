<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-138 — Plan

**Item:** [FEAT-138](../items/FEAT-138-pipelines.md)

Written on 2026-10-14, before pipeline code.

## The pieces

1. **Marked YAML and common model** in `spagitty-core/src/pipelines/`: read working-copy GitHub workflows and GitLab local includes; retain source positions, anchors and references, defaults, inheritance, steps, matrices and dependency edges. Confine all local paths to the repository. Remote includes remain opaque and are never fetched.
2. **Simulation** evaluates the proposal's supported expressions, trigger/ref/path filters, GitLab workflow/job rules and manual/environment approval. Unsupported data remains explicitly decided on the host. The changed files come from working-copy versus upstream or a resolved commit range.
3. **Problems** use bundled SchemaStore schemas with an offline JSON Schema validator. Include original licences and pinned source information. Add nearest-key hints and expose GitLab lint as a separately confirmed network call.
4. **Live host model** in `forge/pipelines.rs`: conditional run/job reads, host step states and durations, completed GitHub logs, partial GitLab traces, explicit cancel/rerun writes with named permission requirements. Reuse the existing HTTP boundary, watched requests and release artifact mappings.
5. **Commands and state** resolve real remotes, persist added watched branches and keep the default branch plus open request branches. Reuse the shared forge clock: five seconds while a run is active, sixty seconds otherwise, with hidden-window suppression and stale-reply guards.
6. **Screen** follows Preview and Live artboards: Needs/Stages graph, virtual cards and edges, expandable matrix, stage collapse over 200 jobs, selected-job details and source links, live logs and last-twelve duration bars. Keep existing chrome and the forge rail group above Settings. Omit local execution as approved.
7. **Records and checks** cover both YAML formats, simulation refusal/unknown cases, host fixtures, commands, state and mounted controls. Update the item, index, automated record, manual sweep, screens and changelog before a checked commit and stacked pull request.

## Decisions

- Nothing in a workflow is executed by preview. Secret values are never requested or shown; details list names only.
- Cycles, unknown dependencies and oversized expansions become problems. Matrix expansion is bounded so repository data cannot freeze the UI.
- GitHub's REST job log is presented after completion. Active steps remain visible; partial trace polling is GitLab-only.
- This branch stacks on Releases; no merge is authorized.

## Dependencies

- Reuse the approved `yaml-rust2` parser, already present from Hooks.
- Add `jsonschema` 0.19.1 (MIT, Rust 1.70 minimum), with default features disabled. Preview cannot fetch a schema or read a file named by repository content.
- Add `regex` 1.x (MIT OR Apache-2.0), already present transitively, for bounded linear-time matching of GitLab `=~` expressions. No repository expression is executed.
- Bundle the SchemaStore GitHub Actions schema under Apache-2.0. Its catalog's GitLab CI entry points to GitLab FOSS's schema under MIT Expat. Both are copied from pinned commits with their original licences and source notices.

## What decides it is right

- Fixture workflows exercise anchors, local includes/reuse, inheritance, references, matrices, source lines, cycles, malformed YAML and schema/key problems.
- Simulation fixtures cover the named expression subset, branch/tag/path filters, changed-file modes, approval, and conservative host decisions for unsupported expressions.
- Host fixtures cover both run/job shapes, named permission errors, one-attempt confirmed writes and unavailable active GitHub logs.
- State and mounted tests cover graph selection, matrix expansion, layout, watched branches, run strip, log selection, cancellation and stale replies.
- Format, clippy, Rust suites, frontend types, configured coverage and build pass before the feature commit.
