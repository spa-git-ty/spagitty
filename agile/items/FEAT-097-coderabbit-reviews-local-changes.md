<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-097 — CodeRabbit reviews local changes

**Status:** Open
**Branch:** `feature/FEAT-097-coderabbit-reviews-local-changes`
**Screens:** Working copy (1C), Settings (1K), the command palette.
**Raised by:** the author, through
[`docs/proposals/extensions-and-coderabbit.md`](../../docs/proposals/extensions-and-coderabbit.md),
§9.1–§9.5.
**Depends on:** FEAT-096.

## Problem

CodeRabbit already reviews a working copy from its own CLI, and already plugs
into Claude Code. Neither puts its findings where a Spagitty user is looking at
the change, and neither knows which findings came from which snapshot of the
code.

## Change

`spagitty.coderabbit`, the first official extension, built **only** through
FEAT-096's public contract:

- A native Rust worker under `extensions/coderabbit/worker` that never spawns a
  process itself: it asks the host to run the user's own `coderabbit` (or `cr`)
  executable through declared command profiles, with argv arrays and an
  approved working directory.
- Detection, version and minimum-version checks, `auth status --agent`,
  an explicit sign-in action with a US/EU region, and a connection state that
  tells missing tool, unsupported version, signed out, ready, denied capability
  and failed diagnostics apart.
- Reviews of uncommitted, tracked, untracked-included or committed changes
  against a selected base, with the scope previewed before the first upload,
  streamed progress and elapsed time, cancellation of the whole process tree,
  and a result that is marked stale when the working copy moved during the run.
- An adapter for the CLI's NDJSON `--agent` stream that tells a complete review
  with findings, a complete review without findings, a skipped empty scope, a
  failure after partial findings, and a billing action-required result apart —
  and never reads exit code zero or an empty stream as approval.
- Selected findings sent to an agent as a repair task in a separate farm
  worktree, from a committed snapshot only.

It is bundled with Spagitty and **disabled** until it is set up and consent to
send code to CodeRabbit is given for the repository.

## Scope

Proposal §9.1–§9.5 and the local half of §12.

## Non-scope

Bundling the CLI (it is proprietary and installed by the user), API-key
authentication, `--use-credits` (never added automatically; a paid
continuation is a separate explicit action not built in v1), automatic edits.
The farm gate is FEAT-098 and pull requests are FEAT-099.
