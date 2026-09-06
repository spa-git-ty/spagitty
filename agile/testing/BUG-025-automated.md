<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-025 — Automated checks

Every implementation and review run has one completion owner. An empty claim does not reschedule watchers. Completion applies only to its own run, errors are visible, and cancelled attempts cannot alter a retry.

Regression tests exercise real disposable repositories and bounded child processes.
Record exact results and coverage below; no model credentials or user repositories
are needed for automated checks.

## Results

2026-09-05: all farm unit tests and 43 pipeline tests passed. New assertions prove a pending run is claimed once, 50 duplicate requests start no additional watcher, cancellation stays terminal, and implementation completion automatically collects its review. Workspace Clippy with warnings denied and record tests passed.
