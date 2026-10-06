<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-054 — Automated test record

**Item:** [`agile/items/TASK-054-build-macos-on-a-mac.md`](../items/TASK-054-build-macos-on-a-mac.md)

## What was tested

`tools/release-macos.test.ts`, *the local Mac build signs and checks like the
lane*:

- `APPLE_SIGNING_IDENTITY="-"` exported, and no Apple secret named;
- `set -euo pipefail`, `codesign --verify --deep --strict`, `codesign -dv`,
  `hdiutil verify` and the `CFBundleExecutable` architecture check present;
- no `spctl --master-disable` and no `xattr`;
- `arm64`, `intel` and `all` mapped to their triples, the target added with
  `rustup`, the build run with `--bundles dmg --target`;
- run with bash on a machine that is not a Mac: exit 1, saying it needs a Mac
  and pointing at `docs/BUILD_MACOS.md` (skipped on a Mac);
- the document names the workflow, the `draft/` push, both artifacts and the
  script, and says the build is not notarized.

Also `bash -n scripts/build-macos.sh` — no syntax errors.

## Test command and output

On Windows 11: `bunx vitest run tools/release-macos.test.ts` — 39 passed. The
script was run through Git Bash: exit 1, "build-macos.sh needs a Mac".

## What is not covered automatically

The build itself, which only a Mac can run. The GitHub half is exercised by the
draft run started from this branch; the local half is the sweep.
