<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-040 — Plan

**Item:** [`agile/items/TASK-040-the-macos-download-says-what-it-is.md`](../items/TASK-040-the-macos-download-says-what-it-is.md)

## Approach

The symptom is on a Mac and this repository has none, so the work is arranged
around a line that has to be drawn honestly: **what the repository is
responsible for** against **what only the failing asset can answer**.

Everything on the first side is fixed here and lands in a file something can
read — the policy in a composite action rather than in three workflows, the
checks beside the build where a job already fails loudly, and the joins between
those files in a vitest file beside `record.test.ts`. Everything on the second
side is written as a procedure with the evidence to record, and left unrun. The
report this task came from ranked its own diagnosis explicitly as a ranked
diagnosis rather than a finding, and that distinction is kept.

The verification runs **after the bundle and before the upload**. That is the
only moment when the artefact exists, nothing has been tagged, and a failure
costs a red job rather than a burnt version number — the same argument
`release-assets` already makes about AppImage update information.

## Decisions

- **The identity is a lane's decision, not the configuration's.**
  `"signingIdentity": "-"` in `tauri.conf.json` is the shortest ad-hoc signing
  and the wrong place for it: one file cannot tell a draft from a release, so
  production would silently ad-hoc sign, verify green, and be refused by
  Gatekeeper on every machine but the build runner. `tauri-bundler` reads
  `APPLE_SIGNING_IDENTITY` from the environment, so the composite action is the
  one place the question is answered and three lanes cannot drift into three
  answers. `tools/release-macos.test.ts` asserts the config stays empty of it.
- **Ad-hoc for interim, Developer ID for production, and production *fails*
  without a certificate.** The refusal is the point of the action. Without it,
  a rotated or removed secret publishes an unsigned build under notes that
  claim it was signed — which is precisely the misdescription this task started
  from, moved one level up.
- **Three separate questions, three separate verdicts.** `hdiutil verify` tests
  the container, `codesign --verify` tests the seal against the bytes, `spctl
  --assess` tests this machine's policy. A valid ad-hoc signature passes the
  second and fails the third, by design. Collapsing them into "it's signed" is
  how a build gets described as fine when one of three things is true, so each
  is printed in its own group and the failure conditions differ by lane.
- **`--deep` for verification only.** The matching signing flag is not used and
  must not be. Deep re-signing an assembled bundle after the fact is what
  produces an outer seal covering resources changed after they were sealed —
  the state macOS reports as damaged. A test asserts no `--force` reaches
  `codesign` here.
- **The architecture is read out of the built binary**, not assumed from the
  runner. A lane that dropped its `--target` builds the host's architecture
  under the other architecture's filename, and that download fails on a user's
  Mac with a message indistinguishable from the others.
- **`macos-15-intel`, and both architectures in all three lanes.** `macos-13`
  is retired. The larger defect was the absence: gate 5 and `prerelease.yml`
  each ran one `macos-latest` job, which is Apple silicon, so no published
  release and no alpha has ever carried an Intel download and nothing reported
  a problem — an absent artefact fails no check.
- **`SHA256SUMS-*.txt`, one per build machine.** A single `SHA256SUMS` written
  by four runners into a shared artifact directory collides exactly the way the
  two `.dmg` names did. It exists because "are these the bytes we built?" is the
  first question a damaged-app report has to answer and, until now, the honest
  answer was that nobody could know.
- **The quarantine instruction is removed from the install path and kept as a
  diagnostic in `docs/ci.md`,** labelled as a bypass and explicitly excluded
  from any acceptance test. It repairs nothing; as *the* documented step it
  taught every Mac user to disarm the check that would have caught a genuinely
  broken file.
- **No YAML parser added for the test.** The assertions match text in files
  this repository writes by hand. A dependency justified by one test file is a
  dependency to explain in every future handoff.

## Files

- `src-tauri/tauri.conf.json` — the `bundle.macOS` block
- `.github/actions/macos-signing/action.yml` (new)
- `.github/actions/macos-verify/action.yml` (new)
- `.github/actions/release-assets/action.yml` — checksums
- `.github/workflows/gates.yml`, `prerelease.yml`, `draft-release.yml`
- `tools/release-macos.test.ts` (new)
- `docs/ci.md`, `CHANGELOG.md`
- `agile/` — this set and the index row

## Steps

1. The `bundle.macOS` block, with no identity in it.
2. The two composite actions.
3. The three lanes: architectures, policy, verification, suffixes.
4. Checksums in `release-assets`, exercised against a fake bundle tree.
5. `tools/release-macos.test.ts`, each load-bearing assertion run against the
   state it was written for.
6. `docs/ci.md`, the changelog, this set and the index row.

## Risks and rollback

- **The production branch has never run.** No Apple Developer account exists, so
  the Developer ID path is written, tested for the parts that are files, and
  unproven. `SWEEP-010` is what proves it. The interim branch is the one every
  lane takes today.
- **Gate 5 now refuses to build macOS without a certificate**, which means a
  `main` release currently publishes no macOS download at all where it
  previously published an unsigned one. That is the deliberate half of the
  trade: the draft and alpha lanes still produce a Mac download and now say
  truthfully what it is. Reverting is a one-word change from `production` to
  `interim` if the author decides otherwise.
- **`macos-15-intel` availability.** If GitHub retires it in turn, the two
  assertions in `tools/release-macos.test.ts` will not know — they refuse
  *known* retired labels rather than confirming a live one, because a test that
  queried GitHub would fail on a network. The sweep carries the real check.
- Rollback is reverting the workflow and action edits. Nothing in the
  application changes behaviour; the only shipped change is a
  `minimumSystemVersion` that was already the bundler's effective default.
