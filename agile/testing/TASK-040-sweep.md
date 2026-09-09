<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-040 — Manual sweep

**Item:** [`agile/items/TASK-040-the-macos-download-says-what-it-is.md`](../items/TASK-040-the-macos-download-says-what-it-is.md)

**This sweep is the diagnosis, and it is unrun.** Everything below needs a Mac
and a published asset; this repository has neither. Nothing in the item claims
to have identified why the author's particular download was called damaged, and
this document is why: the evidence that would settle it is listed here, not
gathered.

## Before touching anything, record the case

A diagnosis that starts after the file has been "fixed" answers a different
question. Download the **published** asset with a browser, on a clean user
account or a VM, so quarantine behaves the way a user's download does. **Do not
strip quarantine and do not re-sign before this table is filled in.**

| Fact | Value |
| --- | --- |
| Release URL and tag | |
| Asset basename | |
| SHA-256 of the downloaded file | |
| The `SHA256SUMS-*.txt` line for that asset | |
| macOS version (`sw_vers`) | |
| Architecture (`uname -m`) | |
| The exact warning text, verbatim | |
| Which double-click produced it — the **`.dmg` before it mounts**, or **`Spagitty.app` after it is installed** | |

**The last row decides which half of the investigation is relevant.** A DMG is a
container and an app is signed code; `hdiutil` and `codesign` answer different
questions, and a warning on the image is not evidence about the app inside it.

## Read-only diagnostics

Run against both copies — the app inside the mounted image and the installed
one — because they can differ, and the difference is itself a finding. Take
`CFBundleExecutable` from the plist rather than assuming the binary's name.

```sh
sw_vers
uname -m
shasum -a 256 ~/Downloads/Spagitty*.dmg
hdiutil verify ~/Downloads/Spagitty*.dmg
xattr -l ~/Downloads/Spagitty*.dmg
codesign -dvvv /Applications/Spagitty.app
codesign --verify --deep --strict --verbose=2 /Applications/Spagitty.app
spctl --assess --type execute --verbose=4 /Applications/Spagitty.app
xattr -lr /Applications/Spagitty.app
plutil -p /Applications/Spagitty.app/Contents/Info.plist
file /Applications/Spagitty.app/Contents/MacOS/spagitty
```

On macOS versions that have it, add `syspolicy_check distribution`. For a
notarized release, add `xcrun stapler validate` on both the app and the image.

**Interpreting them separately is the point.** `hdiutil` tests the container.
`codesign --verify` tests whether the seal still matches the bytes. `spctl`
tests whether this machine's policy will run it. A valid ad-hoc signature passes
the middle one and fails the last, and that is not a defect.

## Tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | A draft cut from this branch, and an Apple silicon Mac | Download `*-macos-arm64.dmg` with a browser. Compare its SHA-256 against `SHA256SUMS-macos-arm64.txt`. Then `hdiutil verify` it | The checksums match and the container verifies. A mismatch here ends the investigation: the bytes are not the bytes that were built, and nothing about signing is relevant yet. | High | |
| SWEEP-002 | SWEEP-001 passed | Mount the image and run `codesign -dvvv` on the app inside it | It reports a signature. **`code object is not signed at all` is the pre-TASK-040 state and a failure of this task.** For an interim build the identity is ad-hoc (no authority chain). | High | |
| SWEEP-003 | SWEEP-002 passed | Drag the app to Applications and double-click it | The **unidentified developer** dialog, not a damaged-app dialog. If macOS still says damaged on an app that `codesign --verify` accepts, that is a genuine finding and the item's diagnosis is wrong — record everything and keep investigating. | High | |
| SWEEP-004 | SWEEP-003 showed the expected dialog | Open System Settings → Privacy & Security, press **Open Anyway**, confirm | Spagitty opens, and opens again on subsequent double-clicks without asking. Record the exact macOS version this was confirmed on: the flow has moved between releases and the notes name it. | High | |
| SWEEP-005 | An Intel Mac, and the same draft | Repeat SWEEP-001 to SWEEP-004 with `*-macos-x86_64.dmg` | The same results. **This is the first Intel Mac download this project has ever produced from a green lane**, so a failure here is expected to be new rather than a regression. | High | |
| SWEEP-006 | Both downloads | On each Mac, run `file` on the other architecture's app | Each reports the architecture its filename claims, and the mismatched one refuses to launch with an architecture error rather than a damaged-app warning. The point is to know what the *other* failure looks like, so the two are never confused again. | Medium | |
| SWEEP-007 | A draft run in GitHub Actions | Open the `Verify the macOS artefact` step on both macOS jobs | Four groups, each with its own verdict: container verified, `codesign` accepted, Gatekeeper **rejected** with the notice explaining that this is expected for a build that is not notarized, architecture confirmed. A green step with an `spctl` acceptance on an ad-hoc build would mean the assessment did not run. | High | |
| SWEEP-008 | The author's original failing download, if it still exists | Fill in the case table above against it, then run the read-only diagnostics | **This is the ticket the whole task came from and the only one that answers it.** The item ranks quarantine-plus-no-signing as most plausible; an absent or invalid ad-hoc signature, resources changed after signing, an incomplete download and a malformed DMG are the alternatives. Do not claim a root cause without this. | High | |
| SWEEP-009 | Any published release from before this change | `codesign -dvvv` on its app | `code object is not signed at all`. **This is correct and is not a bug to fix**: Amendment 14 forbids moving a tag, so every release up to 0.7.0 stays as it is. The ticket exists so that "the old download is still damaged" is a known fact rather than a new report. | Low | |
| SWEEP-010 | An Apple Developer account, and the certificate and notarization secrets added to the repository | Merge a shipping change to `main` and watch gate 5 | The signing step reports Developer ID, the verify step reports Gatekeeper **accepting** the build, and `stapler validate` passes on both the app and the image. Then download the published `.dmg` on a clean machine: it mounts and the app opens through the ordinary first-open confirmation **with no damaged-app warning**. That sentence is the production acceptance criterion. | High | |
| SWEEP-011 | The secrets from SWEEP-010 removed again | Merge a shipping change to `main` | Gate 5 **fails** in the signing step, with the message naming the missing secrets. It does not build, and it does not publish an unsigned macOS asset. | High | |

## Negative paths this sweep deliberately covers

- **SWEEP-009** is the honest one. Anybody testing this will reach for the
  release they already have, which is the one that cannot pass.
- **SWEEP-011** is the only proof that the production refusal is real. A policy
  that has never refused anything is an intention.
- **SWEEP-006** exists because the whole task is about two failures that look
  alike. Knowing what the *architecture* failure looks like is what stops the
  next report being misfiled as this one.

## What must not appear in any of these tickets

`xattr -d com.apple.quarantine`. It is in the diagnostics above as a *reading*
(`xattr -l`), never as a step that changes the file. Stripping quarantine
silences the check; it does not repair anything, and a build that needs it to
open has not passed. Disabling Gatekeeper globally is likewise not a step here.
