<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-054 — Plan

**Item:** [`agile/items/TASK-054-build-macos-on-a-mac.md`](../items/TASK-054-build-macos-on-a-mac.md)

## Approach

The script repeats, on one machine, what the draft lane spreads over its
actions: `APPLE_SIGNING_IDENTITY=-` from `macos-signing`; `bun run tauri --
build --bundles dmg --target <triple>` from the workflow (one `--`, for bun);
`codesign --verify --deep --strict`, `codesign -dv`, `file` on the
`CFBundleExecutable` and `hdiutil verify` from `macos-verify`. It searches both
`target/` and `src-tauri/target/` as `release-assets` does, and clears the
target's old `dmg/` and `macos/` first so last time's image is never reported
as this one.

The document points at what exists — the `draft/**` trigger, the
`draft-macos-*` artifact names, the `-macos-arm64`/`-macos-x86_64` release
names `release-assets` gives — rather than describing a lane of its own.

## Files

| File | Change |
| --- | --- |
| `scripts/build-macos.sh` | New, executable. |
| `docs/BUILD_MACOS.md` | New. |
| `README.md` | Links it. |
| `tools/release-macos.test.ts` | The script's signing, checks, targets and refusal; the document's two ways. |

## Risks and rollback

- Only a Mac can run the build itself; the sweep carries that. Rollback is a
  revert.
