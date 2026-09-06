<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-037 — Plan

**Item:** [`agile/items/TASK-037-updates-that-actually-arrive.md`](../items/TASK-037-updates-that-actually-arrive.md)

## Approach

Everything here is a fact about the build lanes, and the build lanes are the one
place this repository cannot run a test. So the work is arranged so that each
decision lands in a file something *can* read: the update information in a
composite action rather than three workflows, the checks in `release-assets`
where a build already fails loudly, and the joins between files in a vitest file
beside `record.test.ts`.

The verification is deliberately at the end of the build and not at the start of
the release. An AppImage with no update information runs perfectly and passes
every other gate; the only moment it is distinguishable is when the file exists
and nothing has been tagged yet.

## Decisions

- **`$UPDATE_INFORMATION`, not a Tauri setting.** There is no Tauri setting.
  `tauri-bundler`'s `linux/appimage/linuxdeploy.rs` runs `linuxdeploy --output
  appimage` without clearing the environment, and
  `linuxdeploy-plugin-appimage` reads `$UPDATE_INFORMATION` and passes it to
  `appimagetool -u`. The environment is the whole interface.
- **`amd64` rather than `x86_64`.** Argued in the item. The short version: the
  glob has to match the file that is actually attached.
- **Hard-coded owner and repository, not `github.repository`.** The same
  reasoning `update.rs` gives for its endpoint — a string that authorises
  replacing a binary must not be resolved from context. A fork build would
  otherwise aim its users at the fork.
- **`latest`, which is GitHub's non-pre-release endpoint.** The same one the
  in-app check reads, so the two cannot disagree about what "newer" means. An
  alpha updates onto the newest stable build rather than sideways onto another
  alpha.
- **The pair is never renamed.** `release-assets` skips `.AppImage` and
  `.AppImage.zsync` when it appends an architecture suffix. The `.zsync` names
  the AppImage in its own header and resolves it relative to its own download
  URL, so both have to reach the release exactly as `appimagetool` wrote them.
- **A `tag` job in `prerelease.yml`.** The build has to be stamped with the tag,
  so the tag has to exist before the build — the shape `draft-release.yml`
  already uses. It also moves the reused-alpha check ahead of three platform
  builds instead of after them, and removes the duplicate arithmetic from the
  publish job.
- **Gate 5 computes its own tag from the manifest** rather than waiting on gate
  6. Both read the same file, so neither has to tell the other, and gate 5
  genuinely runs first.
- **No build script for `SPAGITTY_RELEASE`.** `option_env!` is tracked by cargo
  since 1.46 — checked, not assumed: setting the variable recompiles
  `spagitty-core` on a warm target directory. So `Swatinem/rust-cache` cannot
  serve a build stamped with a stale tag, and a `cargo:rerun-if-env-changed`
  shim would be cargo-cult.

## Files

- `.github/actions/appimage-update-info/action.yml` (new)
- `.github/actions/release-assets/action.yml`
- `.github/workflows/gates.yml`, `.github/workflows/prerelease.yml`,
  `.github/workflows/draft-release.yml`
- `crates/spagitty-core/src/update.rs`
- `tools/updates.test.ts` (new)
- `docs/ci.md`, `CHANGELOG.md`, the version in the four manifests
- `agile/` — this set and the index row

## Steps

1. The composite action, and the three lanes wired to it.
2. Collection and refusal in `release-assets`.
3. `SPAGITTY_RELEASE` in gate 5 and the prerelease lane; the `tag` job.
4. `update.rs` spelling, and the module note about what was inert.
5. `tools/updates.test.ts`, each assertion run against the broken state first.
6. `docs/ci.md`, changelog, version, this set and the index row.

## Risks and rollback

- **The lanes prove themselves only on a real run.** The AppImage build was
  reproduced locally — `appimagetool -u` against a throwaway AppDir, then the
  collection script run verbatim against the result, in the pass and in all
  three failure modes — but a workflow is not executable here. The sweep carries
  what a real run has to confirm.
- **`linuxdeploy-plugin-appimage` is downloaded per build**, and Tauri falls
  back to an older built-in copy if the download fails. The built-in copy is old
  enough that its handling of `$UPDATE_INFORMATION` is worth not assuming — which
  is why `release-assets` checks the built file rather than trusting the flag was
  honoured.
- Rollback is reverting the workflow edits. The action and the test are inert
  without them, and nothing in the application changes behaviour.
