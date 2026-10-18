<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-140 — Automated test record

**Item:** [FEAT-140](../items/FEAT-140-home.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| The overview | `crates/spagitty-core/src/home.rs` | An address reads as host and path (scp, https with a user, a trailing slash, ssh with a port, a Windows path left alone); a licence is known by its opening words; unreleased entries counted by heading, continuations and nested items not counted; no Unreleased heading is no count, not zero; a pre-release is not the latest version; the overview of the woven fixture (remote, licence, version, default branch, three branches, drift of a merged branch, six commits in sixteen weeks); an empty repository's overview is empty. |
| Needs you | `src/lib/home/home.test.ts` | The six kinds in the handoff's order; each row worded as the handoff words it; a draft, a pull request that asks nothing and a working task are left out; the longest waiting first within a kind; the chips' filters and counts; the summary and ages. |
| The cards | `src/lib/home/home.test.ts` | Sixteen weeks of seven days with today in the last column; five steps of heat; the next version by the changelog's rule; branch tags (checks failing, pull request, farm agent, release); the drift bar's scale and floor; pull requests ordered and named; fourteen runs per workflow, newest on the right. |
| The screen | `src/lib/home/home.test.ts` | With no repository it asks for one; with one, the header, Needs you, Branches, Working copy, Releases and the farm card are drawn from what was read. |
| Rail and launch | `src/lib/nav.test.ts`, `src/lib/chrome/chrome.test.ts`, `src/lib/session.test.ts` | Home first, its own group, a divider before the Farm; Home is never hidden; every launch lands on Home with the stored commit still wanted, and stays put when Home is already up. |

## Not covered

- Not looked at in the running application; the author is checking it.
- The forge reads (pipelines, releases, assigned issues) are not run against a host.

## Results — 2026-10-18

- `cargo test -p spagitty-core --lib home::`: 7 passed, 0 failed (Windows).
- `bunx vitest run src/lib/home`: 15 passed, 0 failed.
- `bunx vitest run src/lib/session.test.ts src/lib/nav.test.ts src/lib/chrome`: all passed.
- `bun run check`: 0 errors, 0 warnings.
