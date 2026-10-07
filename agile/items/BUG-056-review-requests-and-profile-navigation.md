<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-056 — Review and Pull requests align, and profiles open in Settings

**Status:** Open — implementation, automated checks and browser sweep passed; not merged.
**Screens:** chrome, 1R, 1H.
**Raised by:** the author, 2026-10-07, resuming the interrupted changes to the shared header, no-account state and Manage Profiles link.

## Change

- Review and Pull requests use `ScreenHead`, with the screen title, repository pill, details and actions in the same layout.
- Review shows the current repository and removes This repo / All my repos. An inbox mounted after an older all-repository scope returns to repository scope.
- Both screens use `EmptyState` to centre empty and error messages in the window. Missing-account responses use the same neutral message and Settings → Accounts action; other errors keep the backend's explanation.
- Manage Profiles uses SvelteKit navigation to Settings → You, preserving the session.

## Acceptance criteria

- Both headers show `owner/name` in the same repository pill.
- Review offers no scope controls and refreshes the current repository.
- Missing-account responses and disconnected states show *No account is connected* with an in-app Settings → Accounts action; unsupported remotes and other errors remain distinct.
- Manage Profiles opens `/settings#you` without a document reload.
- Frontend tests, type checks and the production build pass. Visual results are recorded in the sweep.

