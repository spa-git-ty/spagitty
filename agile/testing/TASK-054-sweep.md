<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-054 — Manual sweep

**Item:** [`agile/items/TASK-054-build-macos-on-a-mac.md`](../items/TASK-054-build-macos-on-a-mac.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK054-01 | A Mac with the tools in docs/BUILD_MACOS.md | 1. The doc's commands from a fresh clone, `arm64` | The script prints `target/aarch64-apple-darwin/release/bundle/dmg/Spagitty_<version>_aarch64.dmg`; `codesign` reports `Signature=adhoc` | P1 | |
| SWEEP-TASK054-02 | As 01 | 1. `./scripts/build-macos.sh intel` | An `_x64.dmg` whose app `file` reports x86_64 | P2 | |
| SWEEP-TASK054-03 | The `.dmg` from 01 on an Apple silicon Mac | 1. Open it 2. Drag Spagitty to Applications 3. Open it | The unidentified-developer dialog, then Open Anyway works; not "damaged" | P1 | |
| SWEEP-TASK054-04 | GitHub | 1. Push to `draft/…` as the doc says | Artifacts `draft-macos-arm64` and `draft-macos-x86_64`, and a draft release carrying both `.dmg`s | P1 | |
