<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-032 — Approved farm reliability execution plan

The author approved this plan on 2026-09-05 by requesting its execution after
the canonical amendments update. Target a reliability release; use 0.5.1 only
if the final change preserves documented contracts.

## Baseline

Merge origin/main into this work branch, then land through a reviewed PR into
dev. Preserve existing user files. The sweep baseline passed 2,460 frontend
tests, 938 Rust tests, type checking, formatting, Clippy, and both frontend and
native builds. Frontend branch coverage was 68.73% against a 65% configuration.

## Ordered implementation

1. Cancellation: retain an accessible cancellation handle while a waiter owns
   the session; stop descendants on Unix and Windows; prevent late completion
   from reviving cancelled work. Cover individual tasks and the whole farm.
2. Watchers: claim each run exactly once, follow review as well as implementation,
   stop duplicate watcher churn, and report completion failures.
3. Verification: drain both pipes continuously with bounded retention; make
   deadlines and cancellation terminate descendants; stop after the first failure.
4. Merge gates: automatic merges require passing verification, successful
   independent review, permission, and evidence for the current commit. Validate
   task state and the intended destination before mutation; serialize merges.
5. Coverage: measure maintained route components, restore frontend thresholds to
   at least 70%, retain Rust's 70% line floor, and add meaningful lifecycle and
   cross-platform assertions rather than exclusions or padding.
6. Documentation: reconcile architecture, screens, concurrency, coverage,
   amendments references, and roadmap with the resulting code.
7. Full workflow: run an isolated second repository through planning, dependencies,
   execution, cancellation, restart, failed checks, rejection, and conflicts.
   Headless testing comes first; a human desktop sweep or explicit handover is
   required before agent-driven UI testing.
8. Release: complete acceptance criteria, reviews, conflict resolution, ordered
   License → Quality → Tests → Security → Build → Release gates and installer
   smoke tests. Prepare a release PR and return release changes to dev.

## Working rules

Each item has its own branch from dev, item, plan, automated results, and manual
tickets. Add regression tests and changelog entries with the fix. Keep the
existing architecture and feature set. Preserve worktrees and commits during
recovery. Any dependency addition explains its purpose and license.

## Risks and rollback

Concurrency changes must not hold service locks while waiting for processes.
Automatic merge evidence must be invalidated by changed work. Existing farms
must remain loadable; absent evidence after restart requires fresh checks.
Revert individual item PRs to roll back; never discard authored work or move tags.
