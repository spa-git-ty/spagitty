<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-026 — Verification keeps draining and can be stopped

**Status:** Open
**Branch:** `bugfix/BUG-026-verification-output`
**Screens:** Farm (1Q).

## Problem and reproduction

A verification command that emits more than pipe capacity blocks while the parent polls for exit without reading output. A one-MiB output command reproduced the stall. Timeout kills only the parent, and verification has no cancellation path.

## Scope and acceptance criteria

Noisy checks finish; retained output stays bounded and valid UTF-8. Timeout and cancellation stop descendants. Cancelled tasks cannot advance into review, and commands after a failed or stopped check never start.

## Non-scope

New product features, unrelated formatting, and destructive cleanup of authored work.
