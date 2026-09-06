<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-027 — Require current evidence before automatic merging

**Status:** Open
**Branch:** `bugfix/BUG-027-verified-merges`
**Screens:** Farm (1Q).

## Problem and reproduction

Automatic mode sends an unverified task through review and merges it because approve checks only autonomy. A reviewer can exit unsuccessfully after emitting approval. Merge acts on whichever branch is currently checked out and validates task transitions after Git mutation.

## Scope and acceptance criteria

Automatic merges require passing current checks and independent successful review, merge permission and the original destination. Failed or missing evidence prevents mutation. Invalid task states are rejected before Git runs; concurrent merges serialize and conflicts preserve work. Existing farms load with no trusted evidence until rechecked.

## Non-scope

New product features, unrelated formatting, and destructive cleanup of authored work.
