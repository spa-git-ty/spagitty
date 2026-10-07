<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-056 — Automated test record

**Item:** [BUG-056](../items/BUG-056-review-requests-and-profile-navigation.md)

| Suite | Cases |
| --- | --- |
| `src/lib/chrome/chrome.test.ts` | Clicking Manage Profiles calls in-app navigation, closes the menu, preserves the document URL and does not reopen the repository. |
| `src/routes/review/page.test.ts` | Repository pill and signed-in details; no scope controls; older scope reset and repository refresh; backend and disconnected no-account states; Accounts recovery; unsupported remotes and lookup errors remain distinct. |
| `src/routes/requests/page.test.ts` | Repository pill; backend and disconnected no-account states; Accounts recovery; other host and repository lookup failures stay visible. |
| `src/lib/review/store.test.ts` | Preserved cross-repository reads, missing clone, lookup failure, clone opening and host-specific no-account behavior. |

## Results — 2026-10-07

- Focused regression and record checks: 963 passed across five suites.
- Complete frontend suite: 3,505 passed across 171 suites.
- `bun run check`: Svelte reported zero errors and zero warnings; the extension SDK TypeScript check passed.
- `bun run build`: production build passed.
- `bun run coverage`: passed the configured 70% floors. Statements 81.77%, branches 72.18%, functions 79.78%, lines 84.74%.
- `git diff --check` passed; no unmerged paths.

The system Node shim had no active version, so checks used the bundled Node executable on the process PATH. The first full suite selected Windows' WSL `bash` shim and failed the macOS packaging script check; selecting `C:\Program Files\Git\bin\bash.exe` on the process PATH made that check and the complete rerun pass. No machine configuration or dependencies changed.
