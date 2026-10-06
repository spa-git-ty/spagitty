<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Extensions and CodeRabbit — continuation review

## Scope and provenance

Resumed Claude’s feature/FEAT-097-coderabbit-reviews-local-changes checkout at 821b974 in C:/Users/maref/Projects/spagitty-extensions. Read the author/Claude conversation and the full extensions-and-coderabbit proposal; amendments were excluded at the author’s request. Preserved Claude’s implementation and completed the farm, GitHub PR and distribution portions in the same checkout. Changes are local and uncommitted.

## Implemented and reviewed

The farm owns its supplemental provider interface, policy, durable evidence and repair counts. Desktop composition uses the generic host. Required checks run at manual and automatic merge boundaries and validate clean committed work, exact task/policy/configuration, current provider availability and a fresh post-read task snapshot. Supplemental results never replace independent AgentId review or existing automatic verification. Selected findings retain their identity in the existing rules-first repair request.

GitHub snapshots and top-level discussion writes use the existing core HTTP seam and desktop account/keychain. Actor/app identity, exact revisions and incomplete data remain explicit. The worker displays discussion, inline findings, reviews and checks. Comment delivery means requested; ambiguous or malformed delivery retains uncertainty and never silently retries. Other forges are rejected.

Native target-specific workers are staged before Tauri bundling. The Windows package has its worker under the resource extension directory. macOS uses Tauri sidecars so the worker follows its app signing keychain. Native extensions remain trusted user processes; the CLI is separately installed and owns its credentials.

## Review findings fixed

1. Inactivity expired operations while the host was doing their own tool work. Owned callback guards suspend inactivity for concurrent callbacks while preserving absolute deadlines and cancellation.
2. Tool completion could race the absolute deadline supervisor and publish success. Completion itself now rejects an elapsed deadline.
3. PR confirmation cancellation and an in-flight metadata read could still lead to a comment POST. Confirmations expire visibly, and cancellation is checked again immediately before the sole write.
4. Old scope/start/history responses could replace another repository’s UI state. Generations reject late completions and failures.
5. Unicode SDK frames were bounded by character count rather than UTF-8 bytes. Both receive and send now use byte limits and remove failed callback waiters.
6. Uncertain or malformed comment receipts could enable duplicate requests after a crash. Uncertainty is persisted before POST and survives unreadable receipts until a complete refresh.
7. Windows test executables needed Common Controls v6. A single linked manifest supports tests and the production app without conflicting with Tauri resource embedding. Windows path tests compare canonical paths consistently.
8. Resource packaging built the wrong set of worker files and macOS signing needed a sidecar. Only the selected native package is staged, retaining the original macOS window settings.

This is a source/implementation review by the continuing agent, not an independent reviewer’s approval.

## Verification

| Check | Result |
| --- | --- |
| Frontend type/SDK check | 0 errors and 0 warnings |
| Frontend full coverage suite | 145 files / 3,147 tests passed; 70.01% branches, 81.66% lines |
| Linux full Rust workspace | 1,231 tests passed; opt-in foreign package ignored here and separately passed on Windows |
| Rust line coverage | 79.76%; required floor 70%; final full workspace passed |
| Windows desktop tests | 92 passed |
| Windows farm supplemental tests | 12 passed |
| Windows real host/worker integration | 20 passed |
| Public hello extension harness/package/real host | 2 author tests plus conformance; import/run/disable/uninstall passed |
| Formatting / strict workspace Clippy | Passed, -D warnings |
| Rust dependency gates | Advisories, bans, licenses and sources passed with existing configuration |
| JS production licenses | Passed the CI allow-list |
| JS high-severity audit | Passed; one finding below the high threshold remains |
| Windows production build | Final sources/dependencies built MSI and NSIS; packaged native worker handshake/activate/deactivate/exit passed through the public SDK harness |
| Record/bundle tests after documentation changes | 718 passed |
| Git conflict/whitespace checks | Passed |

Commands: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo llvm-cov --workspace --ignore-filename-regex "(fixture|testing)\.rs" --fail-under-lines 70 --summary-only; bun run check; bun run coverage --maxWorkers 2; bun run tauri build; bun run ext test/pack; cargo test -p spagitty-extensions --test foreign_package -- --ignored; cargo deny check licenses bans sources advisories; bun audit --audit-level high; the production license checker from CI.

Raw local logs are under target/: frontend-coverage.log, rust-coverage.log, tauri-build.log, worker-e2e.log, farm-gate-tests.log, desktop-tests.log, cargo-deny.log and frontend-licenses.log. Detailed feature records are adjacent to this file.

The icon and brand byte-comparison checks fail against the inherited committed assets under the bundled Pillow environment. Assets, icons and both generators are unchanged by this continuation (confirmed with git diff). They were not regenerated as part of the extensions work; this is a remaining quality-gate issue, not a passed check.

## Dependencies

No new application dependency was introduced by this continuation. Compatible patches update devalue 5.9.0 → 5.9.3 and source-map-js 1.2.1 → 1.2.2 to remove high-severity audit findings. Existing Vitest and its coverage adapter move together from 4.1.10 to 4.1.11; the existing test dependencies are retained. LLVM coverage/audit tooling was installed in the existing Linux development environment for verification.

## Remaining evidence

- Live CodeRabbit sign-in, quota/billing and review/repair cycle: unverified; no authenticated CLI was available.
- Live authenticated GitHub PR comment/refresh: unverified; no disposable PR/account was supplied. No external comments were posted.
- Manual keyboard/light/dark and GUI lifecycle sweeps: unverified; automated component/contrast/worker tests are separate evidence.
- Linux production packages and both macOS architectures/signing: unverified locally. The existing production release lane supports Linux/Windows and leaves macOS disabled.
- No commit, merge, push, tag, package publication or independent review was performed. The work items remain Open pending the remaining evidence.
