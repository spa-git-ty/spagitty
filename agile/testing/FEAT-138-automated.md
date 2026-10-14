<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-138 — Automated test record

**Item:** [FEAT-138](../items/FEAT-138-pipelines.md)

## Rust

| What | Cases |
| --- | --- |
| Marked YAML | Anchors, merges, references and source positions survive. Duplicate keys, recursive anchors, reference cycles and multiple documents are refused. |
| GitHub preview | Triggers, branches, tags, paths, conditions, matrices with include/exclude, environments and missing dependencies. Local reusable workflows preserve caller conditions and expand one level. |
| GitLab preview | Local includes, inheritance, defaults, references, stage barriers and manual jobs. Unresolved inheritance and rule order stay unknown. Remote includes are opaque. |
| Changed files | Missing upstream is explicit. Resolved commit ranges work. An in-memory upstream index compares final worktree contents and leaves the real index unchanged. |
| Limits and schema | Bounded matrices and expressions; bundled schemas validate offline. Unknown job keys have nearest-key hints. Secret references expose names. |
| Host mappings | Both hosts' run/job times and states, GitHub step states, manual GitLab runs and named permission errors. Writes refuse missing confirmation before any network call. |
| Commands | Mock application plus a real repository: open-repository requirement, confirmation first, watched-branch persistence, duplicate removal and unknown remote refusal. |

## Frontend

`src/lib/pipelines/pipelines.test.ts` mounts the actual screen and graph while mocking typed API wrappers.

| Cases | What they show |
| --- | --- |
| Preview | Local reads, selection, matrix details, source lines and stale-simulation rejection. |
| Live | Default/request branches, shared-clock cadence, active step states, completed GitHub logs, partial GitLab logs and stale-log rejection. |
| Writes | Confirmation precedes delivery. Cancellation or uncertain delivery never silently retries. Watches are local preferences. |
| Graph | Needs edges, expandable matrices, source selection, viewport bounds, stage collapse and ambiguous-name refusal. |

## Not covered

- Nothing was run in the built application. The [sweep](FEAT-138-sweep.md) is for inspection there.
- No authenticated host reads, writes, lint submissions or log downloads were performed. Host API shapes and refusal paths use fixtures.
- Signed installers, Rust coverage and the full farm/extension suites were not run; those crates are unchanged.
- Unsupported expressions, glob syntax, dynamic matrices and conditional/wildcard/remote includes remain explicitly decided on the host.

## Results — 2026-10-14

- Focused core: 14 passed. Pipeline commands: 2 passed.
- Focused frontend/design/rail: 88 passed; later chrome/pipeline/platform probe: 148 passed.
- `cargo fmt --all -- --check` and workspace clippy with `-D warnings`: clean.
- Final type check: 0 errors, 0 warnings. Production static build succeeded.
- Final frontend coverage after the paged-job refresh fix: 216 files, 4787 tests passed. Statements 82.91%, branches 72.68%, functions 79.58%, lines 86.73%; configured floor 70%.
- Full Rust suite: application 175 passed (1 existing ignored), core 926 passed, process 27 passed (1 existing ignored), 0 failed. Existing ignored tests require authenticated settings or an installed multiplexer.
- Record and pipeline tests after final status: 1127 passed. Final type check remains 0 errors, 0 warnings; final production static build succeeded.

## Dependencies

`jsonschema` 0.19.1 (MIT, Rust 1.70) validates bundled schemas without HTTP/file resolution. `regex` (MIT OR Apache-2.0, already transitive) matches bounded GitLab expressions. New transitive licences were read from Cargo metadata. Original schema licences and pinned sources are bundled and listed in `NOTICE`.

`cargo-deny` 0.20.2: licences, bans and sources passed. Duplicate dependency versions are warnings under the existing configuration.
