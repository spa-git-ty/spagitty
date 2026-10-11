<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-068 — Plan

**Item:** [BUG-068](../items/BUG-068-ordinary-commits-marked-as-conflict-fixes.md)

Make both suspects explicit in `crates/spagitty-core/src/remerge.rs`: add `conflicted` to `Region`, keep only conflicted regions in `conflicts()`, and skip a merge unless exactly one parent is in the target's ancestry. Fixture tests: a merge that changes a line outside any conflict; a conflict resolved merging another branch; a clean merge then an ordinary commit.
