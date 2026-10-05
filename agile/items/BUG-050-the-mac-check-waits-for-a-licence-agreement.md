<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-050 — The Mac check waits for a licence agreement

**Status:** Fixed.
**Branch:** `bugfix/BUG-050-the-mac-check-waits-for-a-licence-agreement`
**Screens:** None; CI, every lane that builds a Mac image.
**Raised by:** the draft release run for TASK-054
([run 37285108606](https://github.com/spa-git-ty/spagitty/actions/runs/37285108606)),
built because the author asked for a fresh Apple Silicon build of everything
done.

## Problem

`tauri.conf.json` sets `bundle.licenseFile`, so every DMG carries the GPL as a
licence to agree to before it opens. `.github/actions/macos-verify` mounts the
image with `hdiutil attach`, which prints the licence and waits for an answer.
On a runner nothing answers: stdin is empty, `hdiutil` reads that as a refusal
and exits 1. Both Mac lanes compiled, signed and passed `hdiutil verify`, then
failed there, so the draft release, which waits for all four builds, was never
cut.

It has never passed. The licence has been in the bundle since the first commit,
and before TASK-040 nothing mounted the image.

## Scope

- The mount answers the licence prompt, as a person opening the download does.
  The licence text goes to a file and is printed only if the mount fails.
- The DMG keeps its licence: what it shows a person is unchanged.
- No changelog entry: nothing a user runs changes.

## Acceptance criteria

- A draft release run's Apple Silicon and Intel jobs pass the verify step and
  the release is cut with both DMGs.
