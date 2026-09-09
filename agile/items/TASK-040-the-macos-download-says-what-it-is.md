<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-040 — The macOS download says what it is

**Status:** Done.
**Screen:** — (CI/CD, and the release notes a person reads before installing).
**Raised by:** the author, reporting that a downloaded macOS build refuses to
open and says the application is **damaged**, not that it is from an
unidentified developer.

## Problem

Three separate defects were folded into one symptom, and the release notes told
the user a fourth thing that is not true.

**Nothing signs the macOS bundle at all.** `src-tauri/tauri.conf.json` has no
`macOS` bundle block, no signing identity and no notarization configuration, so
`tauri-bundler` produces an app with no signature of any kind. An unsigned
binary and an ad-hoc-signed one fail differently: Gatekeeper's "unidentified
developer" dialog — the one with an **Open Anyway** path in Privacy & Security —
is what an app with a *valid but untrusted* signature gets. An app with no
signature, or with a signature whose seal no longer matches its own bytes, is
reported as **damaged** and offered a Move to Bin button instead. Apple's own
warning descriptions distinguish the two; the wording is a statement about the
app's integrity state, not proof that the download was truncated.

**Nothing checks the artefact before it is published.** `release-assets`
verifies AppImage update information and refuses a clash of basenames. It does
not open the `.dmg`, does not look at the app inside it, and does not ask
Gatekeeper anything. So every macOS failure mode this task is about is
invisible until a person downloads the file.

**One of the two Intel lanes is built on a runner that no longer exists.**
`draft-release.yml` pins the Intel job to `macos-13`, whose hosted image GitHub
closed down. The other two lanes never split the architectures at all: gate 5
and `prerelease.yml` both build one `macos-latest` job, which is Apple silicon,
so an Intel Mac has had no download from either lane. This is a build defect in
its own right and it is **not** the explanation for a particular already-
downloaded app being called damaged — an app built for the wrong architecture
fails with a different message.

**The release notes promise a fix that is not one.** Every draft release says
that `xattr -d com.apple.quarantine /Applications/Spagitty.app` opens it.
Stripping the quarantine attribute is a troubleshooting bypass. It does not
repair a broken signature, it is not evidence that the packaging is correct, and
telling every Mac user to run it as the normal installation step teaches them to
disarm the check that would have caught a genuinely corrupted download.

## What this task does **not** claim

It does not diagnose the author's particular failing download. That needs the
asset itself on a Mac — its SHA-256, `hdiutil verify` on the container,
`codesign --verify` on the app, and `spctl --assess` on the policy — and this
repository has no Mac. `agile/testing/TASK-040-sweep.md` is that diagnosis,
written as a procedure with the evidence to record; it is unrun.

What is fixed here is everything the repository can be responsible for: the
bundle is signed rather than not signed, both architectures are built on runners
that exist, the artefact is opened and checked before it is attached to
anything, and the notes describe what was actually done to the file.

## Change

- **`src-tauri/tauri.conf.json`** gains a `bundle.macOS` block with
  `minimumSystemVersion: "10.15"` — stated rather than left to the bundler's
  default, so "wrong minimum OS" stops being one of the unknowns a diagnosis has
  to rule out. The signing identity is deliberately **not** written here; see
  below.
- **`.github/actions/macos-signing`** (new) decides the signing policy for a
  lane and exports it to the build step. Given a `Developer ID` certificate in
  the repository's secrets it configures Developer ID signing plus notarization
  and stapling; given none it falls back to an **ad-hoc** signature
  (`APPLE_SIGNING_IDENTITY=-`) for the interim lanes, and **fails** for a lane
  declared `production`. A production release may not silently become an
  unsigned one.
- **`.github/actions/macos-verify`** (new) runs after the bundle and before the
  upload: `hdiutil verify` on the container, `codesign --verify --deep --strict`
  on the app inside the mounted image *and* on the staged app, `spctl --assess
  --type execute` for the policy verdict, `xcrun stapler validate` when the lane
  notarized, and `file` on the executable to confirm the architecture the lane
  claims. Each of the three questions is reported separately, because they
  answer different things — a valid ad-hoc signature passes integrity and fails
  trust, and reading one as the other is how "it's signed" becomes a false
  claim. It fails the job on a broken container, a broken signature, or the
  wrong architecture; on a notarized lane it also fails on an `spctl` rejection.
- **Every lane builds both architectures on runners that exist.**
  `macos-13` becomes `macos-15-intel`; gate 5 and `prerelease.yml` gain the
  explicit `aarch64-apple-darwin` / `x86_64-apple-darwin` split that
  `draft-release.yml` already had, with the `-macos-arm64` / `-macos-x86_64`
  suffix that keeps their `.dmg` names apart.
- **`release-assets` publishes a `SHA256SUMS` file** beside the assets it
  collects. A user who is told an app is damaged can then find out, before
  anything else, whether the bytes they hold are the bytes that were built.
- **The release notes are rewritten** in all three lanes to say what the
  signing policy of that build actually was, what dialog to expect, and what
  the documented first-open path is. The quarantine-stripping instruction is
  gone from the normal path; it survives in `docs/ci.md` as a diagnostic step,
  labelled as a bypass and explicitly not part of the acceptance test.
- **`tools/release-macos.test.ts`** (new) holds the joins between the config,
  the two composite actions and the three lanes.
- **`docs/ci.md`** gains the macOS signing policy — interim and production,
  what each guarantees, and what neither does.

## Why the identity is not in `tauri.conf.json`

`"signingIdentity": "-"` in the config is the shortest way to ad-hoc sign, and
it is the wrong place for it, because the config is one file shared by every
lane. Written there it is what a production build gets too, and a production
build silently ad-hoc signed is precisely the failure the policy exists to
prevent: it bundles, it verifies, `codesign --verify` passes, and it is refused
by Gatekeeper on every machine that is not the one that built it.

`tauri-bundler` reads `APPLE_SIGNING_IDENTITY` from the environment, so the
decision belongs to the lane, where the answer to "do we have a certificate?"
is known. The composite action is the single place that answers it, so three
workflows cannot drift into three different answers.

## Acceptance criteria

- A macOS build from any lane produces an app with a signature. `codesign -dvvv`
  names an authority; it does not say `code object is not signed at all`.
- `hdiutil verify` passes on every published `.dmg`, and the verification runs
  in the lane rather than on a user's machine.
- Both `arm64` and `x86_64` downloads exist for every lane, from runners that
  are not retired, and `file` on each one reports the architecture its filename
  claims.
- A lane declared `production` fails when signing credentials are absent,
  rather than publishing an unsigned build.
- Every release attaches `SHA256SUMS` covering its own assets.
- No release note instructs a normal installation to strip quarantine.

## Non-scope

- **Notarization is configured but unproven.** No Apple Developer account exists
  for this project, so the production branch of the policy has never run. It is
  written, it is tested for the parts that are files, and `SWEEP-010` is what
  proves it the first time credentials exist.
- **Windows SmartScreen.** The same class of problem with a different authority,
  and a separate decision.
- **A universal binary.** Two downloads was a deliberate choice in
  `draft-release.yml` and this does not reverse it.

## Dependencies

TASK-025 built the release lane. TASK-037 built `release-assets`' refusal
behaviour, which this extends. The report at
`docs/analysis/ui-polish-macos-omarchy-claude-handoff.md` ranked the diagnosis.
