<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-058 — Release verification

**Item:** [Restore the patch release](../items/TASK-058-restore-the-patch-release.md)

- GitHub tag lookup returned no v2.0.1 ref before preparation.
- Reviewed the diff: duplicate BUG-059 through BUG-062 rows are reconciled to
  one Fixed row per item; Cargo.lock changes only workspace package versions.
- Reviewed the 2.0.1 notes: all five pending bug fixes are present, with the
  release-check correction and the existing macOS signing and download policy.
- Reviewed the cache timeout follow-up: only optional caching tolerates failure;
  all license, quality, coverage, security and build checks still block their
  gates. All build lanes use the same limits without changing existing keys.
- After merging, check that Gate 0 enables shipping and gates 1 through 6 pass,
  then verify the v2.0.1 release has all platform downloads: pending.

This correction changes records and release metadata. It does not replace the
manual application sweeps owed by the individual bug-fix items.
