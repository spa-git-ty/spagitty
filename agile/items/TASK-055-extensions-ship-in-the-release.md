<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-055 — Extensions ship in the release

**Status:** Done — merged into `main` on 2026-10-06 in `e819aeb`, whose message called it work in progress. Released since: the 3.3.0 `.deb` carries the worker at `usr/lib/Spagitty/extensions/spagitty.coderabbit/bin/x86_64-unknown-linux-gnu/` (checked 2026-10-19). Installing and running it on each system, and the macOS signature, are owed to the sweep.
**Branch:** `feature/FEAT-097-coderabbit-reviews-local-changes` (continuation).
**Screens:** none.
**Raised by:** the author, through
[`docs/proposals/extensions-and-coderabbit.md`](../../docs/proposals/extensions-and-coderabbit.md),
§13 and Phase 5.
**Depends on:** FEAT-096–FEAT-099.

## Problem

A bundled extension is only bundled if the release build puts its worker where
the host can find it on every target, signs it where the platform requires, and
the pipeline notices when an extension changes. Today the scope job classifies
`src/`, `src-tauri/` and `crates/` as shipping; a change under `extensions/`
would skip the release build entirely.

## Change

- The official worker is built for the target being bundled and placed in the
  application's resources before Tauri bundles them, located at run time
  through the resource API rather than the checkout.
- On macOS the worker is signed with the same identity as the application, so
  notarisation sees no unsigned Mach-O.
- The scope job classifies `extensions/`, `packages/extension-sdk/`,
  `tools/extensions/` and `schemas/extensions/` as shipping.
- Licences, notices, the changelog and the architecture, privacy and testing
  documents say what changed, including that a user-enabled CodeRabbit CLI and
  trusted extension workers can talk to services other than a forge.
- The actual release matrix (Linux and Windows enabled in gate 5, macOS lanes
  commented out there and enabled in the draft lane) is recorded as it is.
