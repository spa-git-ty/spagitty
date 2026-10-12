<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-066 — Extensions refuse to run on Spagitty 2

**Status:** Fixed — on `bugfix/BUG-066-extensions-refuse-spagitty-2`; unmerged.
**Screens:** 1K.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## What happens

Every extension, the bundled CodeRabbit one included, is shown as incompatible:

> It needs Spagitty >=0.8.1, <2.0.0, and this is 2.0.1.

## Cause

The application range in four manifests stops below 2.0.0:

- `extensions/coderabbit/extension.json`
- `extensions/coderabbit/bundle/spagitty.coderabbit/extension.json`
- `examples/extensions/hello/extension.json`
- `packages/extension-sdk/templates/basic/extension.json`

The check is `crates/spagitty-extensions/src/version.rs` (`check`, the sentence at line 128). It did what it was told; nothing failed when 2.0.0 was released, because nothing compares the bundled manifests against the application's own version.

## Acceptance criteria

- The bundled CodeRabbit extension installs and runs on 2.0.1.
- The example and the SDK template admit the current major version, so a new extension does not start out broken.
- A test fails when a bundled manifest's `engines.spagitty` does not admit the version in `package.json`, so the next major version cannot ship with the same fault.
- The decision about what the application range is for is written down: the extension API already has its own range (`extensionApi: ^1.0.0`). Either the application range only gets a lower bound, or every major release widens it. Write the reason in the item when it is fixed.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** the four manifests above, `crates/spagitty-extensions/src/version.rs`, `tools/extensions/bundle.ts` (it builds the `bundle/` copy, so regenerate rather than hand-edit it if it is derived).
- A bundled extension is read from the application's resources and updated with Spagitty (`registry.rs`), so a release with the fixed manifest fixes installed copies too. Check that on an upgrade.
- **Branch:** `bugfix/BUG-066-extensions-refuse-spagitty-2`, with plan, testing documents and a changelog entry when the work starts.

## Fix

The four manifests now give `engines.spagitty` a lower bound only (`>=0.8.1` for CodeRabbit, `>=0.9.0` for the example and the SDK template). **Decision:** what can break an extension is the extension API, which has its own range (`extensionApi: ^1.0.0`); an upper bound on the application only turns every major release into a false *incompatible*, so none is written and `docs/extensions/developing.md` says so. A bundled extension is read from the application's resources each time extensions are discovered (`registry.rs`), so an upgraded Spagitty brings its fixed manifest with it. `crates/spagitty-extensions/tests/bundled_manifests.rs` fails when a shipped manifest does not admit the version in `package.json`.
