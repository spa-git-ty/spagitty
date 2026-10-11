<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-058 — Restore the patch release after the bug-fix merges

**Status:** Open — preparing 2.0.1 on `codex/fix-release-2.0.1`.
**Screen:** CI.
**Raised by:** the failed main workflow after pull request #58.

## Problem

Run `37857852830`, at main commit `9c7964d`, failed four working-record
assertions: BUG-059 through BUG-062 each had both a Backlog and a Fixed index
row. Their item documents all say Fixed. Gate 0 also disabled building and
publishing because the manifests still named the already-released 2.0.0.

## Acceptance criteria

- Each of BUG-059 through BUG-062 has exactly one index row, matching its item.
- Application manifests, the MSI version and workspace lockfile agree on 2.0.1.
- The five pending bug fixes and this release correction have a dated 2.0.1
  changelog section, including the existing macOS signing and download policy.
- Record, version and release tests pass, frontend coverage meets its existing
  thresholds, and the Windows release binary builds.

Publication and the Linux and macOS build results are verified by the existing
main workflow after this correction is merged.
