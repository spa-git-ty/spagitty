<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-059 — Manual Sweep

## Steps Verified

1. Open Requests screen with connected account -> verify skeleton shimmer animation displays while loading.
2. Ensure PR list is full-width with old side panel removed.
3. Click any Pull Request row -> full workspace opens.
4. Check top header elements: PR number, compact title with slow hover marquee, author, time, commit count, checks chip, review status chip, and return back button.
5. Click `CHANGELOG` entry at top of left navigation pane -> PR markdown description renders cleanly in main pane.
6. Expand and collapse "All Changed Files" and "List Of Commits" accordions.
7. Click any commit row directly -> expands to list individual commit files; long titles marquee on hover.
8. Click a file under all changed files or under a commit -> diff renders in center pane.
9. Hover over diff line in Reviewer mode -> `+` icon appears on gutter. Click `+` -> inline draft editor opens.
10. Enter inline comment and save -> draft comment card appears on that line with "Pending Review Draft" badge.
11. Refresh / reload app -> draft comments persist from `localStorage`.
12. Check bottom footer -> displays draft count and "Publish Review" button.
13. Click "Publish Review" -> modal opens with verdict options, summary textarea, and draft count summary. Authors cannot self-approve or request changes.
14. Submit review -> review publishes and new comments appear immediately without leaving the workspace.
15. Toggle mode to Developer View -> draft options switch to thread reply inputs and resolve buttons.
16. Click "Back" -> returns cleanly to Pull requests list view.

**Result:** Obsolete, 2026-10-19 — FEAT-116 moved reviewing to Review and FEAT-094 rebuilt reading a pull request; their sweeps check what this did.
