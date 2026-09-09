<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-040 — Automated tests

**Item:** [`agile/items/TASK-040-the-macos-download-says-what-it-is.md`](../items/TASK-040-the-macos-download-says-what-it-is.md)

## What was tested

`tools/release-macos.test.ts` — 32 assertions over the three build lanes, the
two new composite actions and the Tauri configuration, beside `record.test.ts`
and `updates.test.ts` in the ordinary suite.

Nothing in this task is application code, so nothing here could have been caught
by an existing test. The assertions are joins between files that no compiler and
no gate reads together.

| Test | Asserts |
| --- | --- |
| *lane* names no retired runner image | `macos-13` and `macos-12` appear nowhere. A lane pinned to a closed-down image produces no download at all. |
| *lane* builds Intel as well as Apple silicon | Both Darwin targets are named. **This is the assertion for an absence** — `macos-latest` is Apple silicon, so a lane with one macOS job silently ships arm64-only and nothing reports a problem. |
| *lane* keeps the two macOS downloads apart by name | The `suffix:` input is passed on macOS. Two runners, one product name, one version — and a release asset is keyed by its basename. |
| *lane* uses the shared policy action | A fourth lane cannot be added without deciding a signing policy. |
| *lane* verifies the artefact before uploading it | The action is used **and** appears before `release-assets` in the file. A check that runs after the upload has already handed the file over. |
| gate 5 declares itself the production lane | `lane: production`. |
| *lane* declares itself an interim lane | The draft and prerelease lanes are `interim` and never claim `production`. |
| the policy action refuses a production lane with no certificate | The no-certificate branch tests the lane and exits 1. Without this the refusal is decoration. |
| an interim lane with no certificate still signs ad-hoc | `APPLE_SIGNING_IDENTITY=-`. The difference between the damaged dialog and the unidentified-developer dialog. |
| no signing identity is written into tauri.conf.json | **The tempting shortcut, refused.** A config-level `"-"` ad-hoc signs production too. |
| states the minimum macOS it claims to run on | So "wrong minimum OS" stops being an unknown a diagnosis has to rule out by asking. |
| runs *hdiutil verify* / *codesign --verify* / *spctl --assess* | Three questions that are not one question, each present. |
| confirms the architecture the filename claims | A lane that lost its `--target` builds the host's architecture under the other's name. |
| fails on a Gatekeeper rejection only where the lane notarized | An ad-hoc build is *supposed* to be rejected; a lane failing on that could never be green, and one failing on nothing would ship a rejected notarized build. |
| never re-signs anything | No `--force` reaches `codesign`. Deep re-signing after assembly is what produces the damaged state. |
| no lane offers quarantine stripping as the way to install | The promise this task replaces. |
| says which dialog to expect, and that damaged is a different one | The notes name both dialogs and the Open Anyway path. |
| does not claim the build is notarized | Ad-hoc is not notarization, and the notes may not imply it. |
| publishes checksums for what it publishes | `SHA256SUMS` is produced and mentioned in the notes. |

## Run against the broken state first

As this repository requires of a new test, the load-bearing assertions were
pointed at the defects they were written for.

**The retired runner**, with `draft-release.yml` returned to `macos-13`:

```
 × .github/workflows/draft-release.yml names no retired runner image
   Tests  1 failed | 31 passed (32)
```

One lane fails and the other two stay green, which is correct: only the draft
lane ever named that image.

**The release notes**, with the old macOS paragraph restored verbatim —
"Gatekeeper refuses an unsigned app on a double-click, so open it once from the
right-click menu, or run `xattr -d com.apple.quarantine …`":

```
 × no lane offers quarantine stripping as the way to install
 × says which dialog to expect, and that damaged is a different one
   Tests  2 failed | 30 passed (32)
```

Both failures are the ones intended. The second is the more interesting: the old
text is not merely missing a warning, it names the wrong dialog — it says
Gatekeeper *refuses* an unsigned app, which is true, and then offers the remedy
for a different refusal.

## Exercised outside the suite, because the suite cannot run a workflow

**The collection script**, extracted from `release-assets/action.yml` and run
verbatim against a fake bundle tree, in both naming cases:

| Case | Result |
| --- | --- |
| macOS runner, `SUFFIX=macos-arm64` | `Spagitty_0.7.0_aarch64-macos-arm64.dmg` collected; `SHA256SUMS-macos-arm64.txt` written beside it, listing the renamed file |
| Linux runner, no suffix | Both artefacts collected; `SHA256SUMS-linux.txt` covers both |

The checksum file is written **after** the suffix rename, so it lists the names
that actually reach the release rather than the names the bundler produced. That
ordering is the whole reason it lives at the foot of that script rather than in
a step of its own.

**YAML validity** of all three workflows and all four composite actions was
checked with a parser, since a malformed workflow fails only on GitHub.

## What is not covered, and why

- **Every macOS tool in `macos-verify`.** `hdiutil`, `codesign`, `spctl`,
  `stapler` and `plutil` exist on no runner this repository can reach. The test
  asserts that each is invoked and that the lane-dependent failure conditions
  are written; whether Apple's tools answer as documented is `SWEEP-001` to
  `SWEEP-007`.
- **The Developer ID branch.** No account exists. It is unproven code and the
  item says so; `SWEEP-010`.
- **Whether `macos-15-intel` is available.** A test that asked GitHub would fail
  on a network. Known-retired labels are refused; a live one is confirmed by a
  real run.
- **The author's actual failing download.** It has not been seen. The sweep is
  the diagnosis, and it is unrun.

## Coverage

`vite.config.ts` scopes the denominator to `src/lib/**` and `src/routes/**`, so
a file in `tools/` moves it in neither direction. No Rust or frontend source
changed in this task.
