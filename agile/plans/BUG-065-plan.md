<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-065 — Plan

**Item:** [BUG-065](../items/BUG-065-agent-cli-access.md)

Update the Codex adapter's directory, startup and headless approval arguments.
Persist an opt-in Full Access switch in the machine configuration and expose it
in Settings, preserving it when the review driver adds its read-only default.
Surface the known sandbox startup failure as a failed assignment step.

Detect standard user CLI directories and npm's native Bun runtime, and use the
same PATH for probes and launches. Prefer `omp`; use its print flag. Add an `agy`
provider and adapter, frontend labels and branch parsing, with print and plan modes.
Persist OMP's selected model and optional profile for fresh background launches;
expose them in Settings rather than relying on another terminal's active session.
Persist an opt-in agy auto-approval switch for headless tool requests, preserving
plan mode and surfacing permission denials as failed assignment steps.

Sync completed review file steps into the room's viewed record against their
blobs, keeping completion ids so manual unticks survive replay. Test stale heads,
unfinished files, deletions, configuration persistence and mounted Settings.

Run Rust tests, frontend tests and checks, production builds and authenticated
CLI smoke tests on Windows. Review the diff and open a PR against current main.
