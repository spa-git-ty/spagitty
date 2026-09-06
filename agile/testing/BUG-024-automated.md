<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-024 — Automated checks

A watched task and a collected planner stop promptly, descendants cannot keep writing, terminal state is preserved, and service locks are never held while joining pipes. Existing farm data stays readable. Verification command cancellation is completed with the verification runner item.

Regression tests exercise real disposable repositories and bounded child processes.
Record exact results and coverage below; no model credentials or user repositories
are needed for automated checks.

## Results

2026-09-05: the original watched-cancellation regression failed with a Cancelled
→ Verification transition before the fix. Afterward all farm unit tests and
41 pipeline tests passed, including watched task cancellation, whole-farm
cancellation, collected-planner cancellation, and descendant termination.
Clippy with warnings denied and the frontend record tests passed. Windows
runtime validation is assigned to the platform CI gate; it is not claimed here.

Dependency: a direct Windows-only windows-sys 0.61 dependency exposes job objects
and suspended-thread resumption. It is MIT OR Apache-2.0 and already in Cargo.lock
through Tauri. No Unix dependency was added.
