<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-054 — Build macOS on a Mac

**Status:** Done.
**Branch:** `task/TASK-054-build-macos-on-a-mac`
**Screens:** none.
**Raised by:** the author, 2026-10-05, asking whether the Apple silicon app can
be built on their Windows machine or only on GitHub, with a task brief
(`SPAGITTY_MACOS_BUILD_TASK.md`): GitHub builds of both Mac architectures,
ad-hoc signed and verified, a local build script, and copy-paste instructions.

## Problem

The GitHub half already existed (TASK-040): `draft-release.yml`, `gates.yml`
and `prerelease.yml` build `aarch64-apple-darwin` on `macos-latest` and
`x86_64-apple-darwin` on `macos-15-intel`; `.github/actions/macos-signing`
sets `APPLE_SIGNING_IDENTITY=-` where no certificate is configured;
`.github/actions/macos-verify` mounts the `.dmg` and fails on `hdiutil verify`,
`codesign --verify --deep --strict` or the wrong architecture, and reports
`spctl` without failing an ad-hoc lane on it. What was missing:

- a way to build on a Mac that signs and checks the same way;
- a document saying how to get a Mac build, both ways, in commands.

A Mac app cannot be built on Windows: `codesign`, `hdiutil` and Apple's SDK
are macOS-only, and Tauri does not cross-compile to macOS.

## Scope

- `scripts/build-macos.sh arm64|intel|all`: refuses anywhere but macOS; adds
  missing Rust targets; installs dependencies if absent; signs ad-hoc; builds
  the `.dmg`; verifies the `.app` with `codesign`, its architecture with
  `file`, the image with `hdiutil`; prints the paths; exits non-zero on any
  failure. It never touches Gatekeeper.
- `docs/BUILD_MACOS.md`, linked from the README: GitHub (push to `draft/…` or
  dispatch, where the artifacts and the draft release are) and a Mac (from a
  fresh clone to the `.dmg` path), and how the signature is checked.
- The workflows and actions are unchanged: they already do what the brief
  asks.
- No changelog entry: nothing in the application changes.

## Acceptance criteria

- On a Mac, `./scripts/build-macos.sh arm64` leaves
  `target/aarch64-apple-darwin/release/bundle/dmg/Spagitty_<version>_aarch64.dmg`,
  ad-hoc signed and verified, and prints its path.
- Anywhere else it exits 1 and says to build on GitHub.
- The document's commands work as written.
