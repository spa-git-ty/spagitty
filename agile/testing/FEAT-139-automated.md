<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-139 — Automated test record

**Item:** [FEAT-139](../items/FEAT-139-releases.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| Host shape and writes | `forge/releases.rs` | Host assets, absent GitLab download counts, source archives, **release deletion does not delete tags**, confirmation before sending, per-host publish payloads, **one attempt on uncertain delivery**, GitLab drafts refused by host writer, upload destinations bound to configured hosts and safe file names. |
| Notes and repository | `forge/releases.rs` | Shared Conventional Commit grouping retains request numbers. **Changelog commit uses hooks**, appears in the resulting commit, refuses unrelated work and a stale target. |
| HTTP | `forge/http.rs` | Binary upload and download refuse plaintext. The same configured native TLS and no-redirect agent is used for authenticated transfers; redirected downloads drop credentials. No real file transfer was performed. |
| Commands | `src-tauri/src/releases.rs` | Confirmation before repository access. GitLab drafts and hidden ids persist locally and are isolated by remote repository; local deletion needs no host request. |
| State and mounted screen | `src/lib/releases/releases.test.ts` | Offline saved pages, stale repository replies, local filters and hiding, confirmed publication, independent write errors, separate tag deletion, **changelog resume without a second commit**, per-file failure/retry/progress, GitLab link retry without a second upload, artifact download queue, generated preview, native picker, cancelled publication retains edits, read-only writes absent. |
| Existing behaviour | Navigation and Chrome suites | New Releases row uses the same forge group and existing shell. |

## Not covered

- No running application, visual inspection, native file drop, real host write or authenticated transfer was exercised.
- Live moved-target races, native credential permissions, large downloads and long uploads were not run.
- Rust commands and mounted frontend have not been used together in the application. Linux, macOS, WebKit and CI were not run.
- Rust coverage, cargo-deny and history secret scanning were not run.

## Results — 2026-10-14

- Final core release suite: 6 passed.
- Command suite: 2 passed.
- Focused frontend: 3 files, 156 tests passed.
- `cargo fmt --all -- --check`: clean. Final `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p spagitty -p spagitty-core -p spagitty-process`: 173, 912 and 27 passed, 0 failed. Two existing live tests remain ignored (authenticated agent settings and installed multiplexer attachment). After serializing local preference changes, both release command tests passed again.
- Final `bun run check`: 0 errors, 0 warnings. Final `bun run build`: succeeded with the static adapter.
- Final `bun run coverage`: 215 files, 4774 tests passed. Statements 83.60%, branches 73.51%, functions 80.42%, lines 86.82%; all exceed the configured 70% floor.
- An initial theme-token test found an undefined font token, which was corrected. A later run timed out in the unchanged production-style probe under concurrent compilation; the final full coverage run passed without a timeout change.
- Final application suite after local storage changes: 173 passed, 1 existing live test ignored. Record checks after the final status: 1114 passed. Release controls after the file-removal wording clarification: 12 passed.

## Baseline differences outside this item

The unchanged brand/icon generator differences and optional audit tools are recorded in [FEAT-137](FEAT-137-automated.md). Coverage uses the configured 70% floor rather than the older documentation's 65%.

## Dependencies

None.
