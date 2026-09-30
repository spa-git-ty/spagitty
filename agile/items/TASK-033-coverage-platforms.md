<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-033 — Restore coverage and test supported process platforms

**Status:** Done — merged by pull request #38 on 2026-09-05. Its Windows process job (`execution::tree`, `verification::command`) also passes natively on Windows 11, 2026-09-30.
**Branch:** `task/TASK-033-coverage-platforms`

## Scope and acceptance criteria

Measure all maintained frontend routes and library code, enforce at least 70%
statements/branches/functions/lines, retain Rust's 70% line floor. Add meaningful
headless behavior assertions and Linux/macOS/Windows process containment checks.
No production dependency or unrelated feature changes.
