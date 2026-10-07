<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-053 — Automated test record

**Item:** [`agile/items/BUG-053-what-the-author-found-sweeping.md`](../items/BUG-053-what-the-author-found-sweeping.md)

| Suite | Cases |
| --- | --- |
| `src/lib/graph/lanes.test.ts` | A merge leaves its dot sideways with one turn; a branch comes home sideways; a passing lane still turns in the middle; `turnsAt` for each kind; both turns stay inside their crossing at a squeezed pitch. |
| `src/lib/metrics.test.ts`, `src/lib/graph/density.test.ts` | Column widths and fitting lanes at the new pitch and node. |
| `src/lib/graph/fit.test.ts` | The widest row wins; chips, gaps and `+N` add up; minimum, maximum; each name measured once. |
| `src/lib/chrome/chrome.test.ts` | The strip shows the package's version and no licence label; the SPDX identifier is in its title. |
| `crates/spagitty-core/src/avatars.rs` | A self-hosted GitLab is asked by address with the token and its upload fetched, then answered from disk; the identicon becomes `d=404` and `no_avatar` is none; a refusing GitLab falls back to Gravatar; Bitbucket names the account. |
