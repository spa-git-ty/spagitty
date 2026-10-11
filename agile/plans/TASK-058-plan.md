<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-058 — Plan

**Item:** [Restore the patch release](../items/TASK-058-restore-the-patch-release.md)

1. Reproduce the four record failures and confirm that v2.0.1 is unused.
2. Remove stale duplicate index rows, keeping the Fixed status from each item.
3. Bump package.json, Cargo.toml and tauri.conf.json, including the MSI version,
   then refresh Cargo.lock without changing dependency versions.
4. Record the correction in Unreleased and prepare the 2.0.1 release section
   from the pending fixes, retaining the existing download policy.
5. Run existing record/version/release tests, frontend coverage and type checks,
   and build the Windows release binary before preparing a pull request to main.
