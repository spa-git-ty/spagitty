<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-050 — Manual sweep

**Item:** [`agile/items/BUG-050-the-mac-check-waits-for-a-licence-agreement.md`](../items/BUG-050-the-mac-check-waits-for-a-licence-agreement.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG050-01 | This branch pushed to a `draft/` branch | 1. Open the draft release run 2. Open *build macos-arm64* › *Verify the macOS artefact* | The image mounts; codesign verifies; `file` reports arm64; no GPL text in the log | P1 | |
| SWEEP-BUG050-02 | As 01 | 1. Open *build macos-x86_64* › the same step | As 01, reporting x86_64 | P1 | |
| SWEEP-BUG050-03 | As 01, run finished | 1. Open the draft release | Both DMGs are attached | P1 | |
| SWEEP-BUG050-04 | The arm64 DMG on an Apple Silicon Mac | 1. Open it | The GPL is shown to agree to, then the image opens | P2 | |
