<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-026 — Automated checks

Noisy checks finish; retained output stays bounded and valid UTF-8. Timeout and cancellation stop descendants. Cancelled tasks cannot advance into review, and commands after a failed or stopped check never start.

Regression tests exercise real disposable repositories and bounded child processes.
Record exact results and coverage below; no model credentials or user repositories
are needed for automated checks.

## Results

2026-09-05: all farm unit tests and 44 pipeline tests passed. Tests exercise one MiB on each stream for success and failure, bounded retention, a 50 ms deadline, cancellation, and cancellation during service verification without later checks or review. Workspace Clippy and record tests passed. No dependency additions.
