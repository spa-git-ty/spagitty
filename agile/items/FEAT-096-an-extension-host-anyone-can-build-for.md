<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-096 — An extension host anyone can build for

**Status:** Open
**Branch:** `feature/FEAT-096-an-extension-host-anyone-can-build-for`
**Screens:** Settings (1K), Working copy (1C), Farm (1Q), Pull requests (1H),
the command palette.
**Raised by:** the author, through
[`docs/proposals/extensions-and-coderabbit.md`](../../docs/proposals/extensions-and-coderabbit.md)
(prepared 2026-10-06), §1–§8 and §12–§13.

## Problem

Nothing in Spagitty can be added from outside it. Every review tool, every
integration, every command is a change to this repository, a new Rust enum
variant, a new Tauri command and a new Svelte component. The first integration
the author wants — CodeRabbit — would be the third time the PR screen grows a
host-specific feature by hand, and the first one that is not even a forge.

## Change

A documented, versioned public contract for extensions, and a host that runs
them:

- **A native process per extension**, speaking JSON-RPC 2.0, one compact UTF-8
  JSON message per line on stdin/stdout, with bounded diagnostic stderr. Any
  language that can read a line and write a line can implement it.
- **A manifest** (`extension.json`) with an authoritative JSON Schema, valid and
  invalid fixtures, three independent compatibility dimensions (manifest
  version, extension API version, application version), and capability
  declarations the host enforces on every callback.
- **Contribution points** rendered by the host from data: palette commands with
  availability reasons, actions on the working copy / a farm task / a pull
  request, validated settings, and panels drawn by three host renderers
  (`reviewFindings`, `reviewStatus`, `summary`). An extension ships no HTML and
  no JavaScript into the window.
- **A review provider contract** with a shared review model (snapshot, finding,
  result, gate) that both the Rust host and the frontend use, persisted as
  minimal history under `.spagitty/`.
- **Packages** — a ZIP-based `.spagitty-extension` with an integrity index,
  validated before anything is extracted or run, installed into immutable
  version directories under the application's data directory with an atomic
  pointer, with update-from-file, rollback and uninstall.
- **A developer kit** — a TypeScript SDK, a scaffold, a validator, a protocol
  test harness, a packer and an inspector — and an independently packaged
  `com.example.hello` example that runs on an unchanged build.
- **An Extensions section in Settings** listing provenance, version,
  compatibility, requested and granted capabilities, per-repository
  enablement, settings, diagnostics, update and removal.

The trust model is said plainly everywhere it applies: an extension is trusted
native software running with the user's own privileges. Capabilities limit what
Spagitty's API will do for it; they are not a sandbox.

## Scope

Proposal §6 and §7 in full, §8's shared review model, §12's Extensions section
and the generic entry points, §13's files except those owned by FEAT-097–099
and TASK-055.

## Non-scope

Proposal §4 "Deferred beyond v1": a marketplace, automatic downloads or
updates, a publisher registry, extension dependencies, hot replacement during a
running review, arbitrary HTML/JavaScript UI, a WASM sandbox, an inbound
webhook server. CodeRabbit itself is FEAT-097–099.

## Acceptance

Proposal §16 criteria 1–4, 7, 12 and 13, demonstrated by the tests and sweep
recorded in this item's testing documents.
