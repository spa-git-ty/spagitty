<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-136 — Automated test record

**Item:** [FEAT-136](../items/FEAT-136-issues.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| Host rules | `forge/issues.rs` | Quoted filters, both hosts' parameters, **pull requests excluded**, ETag reuse, system notes as events, linked requests, confirmed writes, correct close verbs, **uncertain delivery never retried**, branch suggestions. |
| HTTP | `forge/http.rs` | Conditional reads refuse plaintext before sending credentials. Existing transport checks still apply. |
| Commands | `src-tauri/src/issues.rs` | No repository, confirmation before credentials, arbitrary remote refused, **branch starts at tracked default tip and keeps untracked work**. |
| State and screen | `src/lib/issues/issues.test.ts` | Provisional cache retained offline; stale repository and selection replies discarded; write errors survive refresh; read-only writes hidden; comment then close confirmed separately; page deduplication; safe Markdown; mounted creation, comments and filters; full brief opens an unready TaskEditor without execution. |
| Shared clock | `src/lib/notifications/watcher.test.ts` | One timer, independent notification and screen periods, **hidden screens do not read**, failures do not stop other subscribers. |
| Existing screens | Navigation, Chrome, Rail settings and Farm suites | New row and separator placement, existing task editor behaviour. |

## Not covered

- No running application, real host writes, real credentials, rate limits or host permissions were exercised.
- Linux, macOS, WebKit, visual matching and native branch/agent interactions were not run.
- The Rust commands and mounted frontend have not been used together in the application. The author owns that sweep.
- CI, Rust coverage, cargo-deny and history secret scanning were not run.

## Results — 2026-10-14

- Focused frontend: 10 files, 264 tests passed. Final Issues and clock suites: 2 files, 13 tests passed.
- Command fixtures: 3 passed.
- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p spagitty -p spagitty-core -p spagitty-process`: 171, 906 and 27 passed; 0 failed. Two existing live tests remain ignored (authenticated agent settings and installed multiplexer attachment).
- After distinguishing system-note ids from linked-request ids, the five core issue tests passed again. The ETag test also checks that an old token's write hint does not survive revalidation.
- `bun run check`: 0 errors, 0 warnings. `bun run build`: succeeded with the static adapter.
- `bun run coverage`: 214 files, 4758 tests passed. Statements 84.09%, branches 74.14%, functions 80.91%, lines 86.93%; all exceed the configured 70% floor.
- `bun run test` after the final records: 214 files, 4759 tests passed.

## Baseline differences outside this item

The unchanged icon and brand generator differences and missing optional audit tools are recorded in [FEAT-137](FEAT-137-automated.md). Frontend coverage uses the configured 70% floor, despite older documentation saying 65%.

## Dependencies

None.
