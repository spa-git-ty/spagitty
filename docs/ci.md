<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# CI/CD

The pipeline is a sequence of gates in a fixed order. A gate that fails stops
the pipeline; nothing downstream runs. Definitions live in
`.github/workflows/`.

**First run: 2026-08-26.** Gates 1 to 4 ran for the first time on pull request
#1 into `dev` — run `32999076513`, nine and a half minutes, all four green. The
workflows landed with the code they gate and then waited six months for a remote
to run on; that wait is over, and they turned out to be right.

**First `main` run: 2026-08-28.** Gate 5 built all three platforms green on the
first merge into `main` (run `33204934146`). Gate 6 failed on that same run:
the runner has no git identity, and `git tag -a` refused with `fatal: empty
ident name`, so `v0.1.0` was never tagged and nothing was published — a halt,
not a broken release. The fix (TASK-025) sets the `github-actions[bot]`
identity in the job and, with it, moved the notes off `--generate-notes` and
onto the changelog, as Amendment 20 requires. The fixed gate 6 has not run yet;
it proves itself on the next merge into `main`.

**One difference worth knowing** between a local run and the runner: `gitleaks`
walks the pull request's merge ref on CI and the branch tip locally, so the
commit counts differ — 115 against 125 on the first run. Both scan everything
they are given.

## The gates

| # | Gate | Runs | Proves |
| --- | --- | --- | --- |
| 0 | Scope | `git diff` of the push / PR against the previous tip | Whether the change ships in the application. Gates 5 and 6 read this; documentation alone is not a release |
| 1 | License | `cargo deny check licenses bans sources`, `bunx license-checker-rseidelsohn@4` over the JS production tree, plus a check that `LICENSE`, `NOTICE` and both manifests still say GPL-3.0-or-later | Every dependency's license is identified and permitted, and nothing conflicts with Spagitty shipping under GPL-3 |
| 2 | Code quality | `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bun run check`, plus `tools/make-icons.py --check` and `tools/make-brand.py --check` (Pillow) | Formatting, lints and types across both languages, and no drift between the brand generators and the committed art |
| 3 | Tests and coverage | `cargo llvm-cov --workspace --fail-under-lines 70`, `bun run coverage` | The suite passes and first-party coverage holds its floor: 70% for Rust, 65% for the frontend |
| 4 | Security | `cargo deny check advisories`, `bun audit --audit-level=high`, `gitleaks` over the diff | No known-vulnerable dependency at the high level or above, no secret in the change |
| 5 | Build | `bun run tauri build` on Linux, macOS and Windows — **main, shipping changes only** | The release build works on every target, not only the one the author uses |
| 6 | Release | tag, artifacts, notes read from `CHANGELOG.md` by `bun tools/release-notes.mjs` — **main, shipping changes only** | The build is published, carries its changelog section as notes (Amendment 20), and is traceable to a commit |

Cheapest and most certain first, so an obvious failure never burns a full build.

**Every lane builds all three platforms.** Gate 5 and the prerelease workflow
each run `ubuntu-latest`, `macos-latest` and `windows-latest`; the draft
workflow builds Linux, Windows, and macOS on two runners — `macos-latest` for
Apple silicon and `macos-13` for Intel, since a build made on one does not run
on the other. Gate 5's macOS build is Apple silicon only, so a published
release currently has no Intel Mac download; that is recorded as an open
question in TASK-025 rather than fixed unwatched on the blocking release path.

**Nothing is signed.** There is no Apple Developer account and no code-signing
certificate in this repository, so macOS Gatekeeper refuses a downloaded build
on a double-click and Windows SmartScreen warns. The draft lane's release notes
carry the one step a Mac user needs; adding real signing is a change to gate 5
and the draft lane together, not to one of them.

## The Linux build can be updated in place

An AppImage is one file with no package manager behind it, so nothing on the
machine knows it can be replaced. FEAT-054 makes a running Spagitty *say* there
is a newer release; this is what makes acting on that cheap rather than a
ninety-megabyte re-download.

`.github/actions/appimage-update-info` exports one string into the build
environment, and all three build lanes use it on their Linux runner:

```
gh-releases-zsync|spa-git-ty|spagitty|latest|*_amd64.AppImage.zsync
```

Tauri has no configuration for this. Its AppImage bundler shells out to
`linuxdeploy`, whose `appimage` output plugin forwards `$UPDATE_INFORMATION` to
`appimagetool -u` — which embeds the string in the AppImage runtime's
`.upd_info` ELF section and writes a `.zsync` control file beside it.
`release-assets` collects both, so every release carries the `.AppImage` and the
`.AppImage.zsync`. `AppImageUpdate` run against an installed build then
transfers the blocks that changed and nothing else.

Three details are load-bearing, and each has a check behind it:

- **`amd64`, not the `x86_64` other projects use.** Tauri names the bundle
  `Spagitty_<version>_amd64.AppImage`, and the last field of the string is a
  glob over the release's asset names. `tools/appimage-update.test.ts` matches
  that glob against the name composed from `src-tauri/tauri.conf.json`, so
  renaming the product fails a test rather than the updater.
- **The pair is never renamed.** The `.zsync` names the AppImage it patches and
  resolves it relative to its own download URL, so the two filenames have to
  reach the release exactly as `appimagetool` wrote them. `release-assets` skips
  both when it disambiguates names with an architecture suffix.
- **`latest` is the newest non-pre-release**, through GitHub's
  `/releases/latest` — the same endpoint the in-app check reads. An alpha
  therefore updates onto the newest stable build rather than sideways onto
  another alpha, and the two ways of hearing about a new version cannot
  disagree about what one is.

`release-assets` refuses an AppImage whose `.upd_info` is empty, whose `.zsync`
is missing, or whose glob does not match the file it just collected. That check
exists because an unupdatable AppImage starts, works, and passes every other
gate: nothing else in the pipeline would ever notice, and the first person to
find out would be a user stranded on a version nobody can move them off.

## What runs where

- **`main`** — gates 1 to 4 on every push. Gates 5 and 6 run only when the
  change *ships*: something under `src/`, `src-tauri/`, `crates/`, or a
  version / lockfile / frontend config that the built application depends on.
  Passing gate 6 publishes the release. A merge of application code into `main`
  is a publish; a README, `docs/`, `agile/` or brand-collateral edit is not —
  those land without rebuilding three platforms or tagging, and ride the next
  version. The scope job (`0 · scope`) is what makes that call.
- **`dev`** — gates 1 to 4, automatically, then the pipeline stops. Building and
  publishing an alpha from `dev` is a manual action: run the `prerelease`
  workflow and give it an alpha number. It produces `vX.Y.Z-alpha.N`, which is
  never a release of `main`.
- **Pull requests** into either branch run gates 1 to 4. Since Amendment 14
  makes the pull request the only path into a protected branch, that is where
  the results are read.

## Rules

- Gates are blocking, not advisory. A red gate is fixed, not bypassed;
  `continue-on-error` is not used to get a merge through, and neither is a
  re-run until it passes.
- **An advisory that cannot be fixed is recorded, not silenced.** `deny.toml`'s
  `[advisories] ignore` list carries accepted risk **by advisory id**, each with
  its crate and its reason, so anything *not* listed still fails gate 4. That is
  the line between recording a risk and switching the gate off, and only the
  first is allowed. A blanket setting — `unmaintained = "warn"`, or dropping the
  check — is the second wearing different clothes.

  Sixteen entries were added in TASK-010, all `unmaintained`, none a
  vulnerability, and eleven of them the GTK3 bindings Tauri links against on
  Linux with no upgrade available. They are deleted when Tauri moves to GTK4.
- The order is fixed. A new check joins an existing gate or becomes a new one in
  the right place — it does not get bolted onto whichever job is convenient.
- The coverage floor is defined once per language: `COVERAGE_FLOOR` in the
  workflow for Rust, `test.coverage.thresholds` in `vite.config.ts` for the
  frontend. They are not the same number — the frontend carries whole screens
  that mount but are not asserted on — and `bun run coverage` fails locally for
  the same reason it fails in CI.
- Tags are never moved. Gate 6 refuses to publish over a tag that already
  exists and tells you to bump the version instead.

## Coverage scope

Only first-party code counts, in either direction — dependencies neither
inflate the number nor deflate it.

- **Rust**: the workspace, with `crates/spagitty-core/src/fixture.rs` excluded.
  It is test scaffolding, and counting a helper that every test exercises would
  lift the figure without any product code being tested.
- **Frontend**: `src/lib/**`. `src/testing/**` is excluded for the same reason,
  and `src/routes/**` is excluded because those files are the screens' shells;
  their logic lives in `src/lib`.

## Running the gates locally

Gates 1 to 3 are what a change is checked against before it is committed:

```sh
bun install --frozen-lockfile              # the dependencies, locked
cargo deny check licenses bans sources     # gate 1, needs cargo-deny
cargo fmt --all --check                    # gate 2
cargo clippy --workspace --all-targets -- -D warnings
bun run check
python3 tools/make-icons.py --check        # gate 2 — brand drift (needs Pillow)
python3 tools/make-brand.py --check
cargo llvm-cov --workspace --ignore-filename-regex '(fixture|testing)\.rs' --summary-only
bun run coverage                           # gate 3
cargo deny check advisories                # gate 4
bun audit --audit-level=high
```

`cargo-deny` and `cargo-llvm-cov` are installed with
`cargo install cargo-deny cargo-llvm-cov`.

## Candidate gates, not adopted

Recorded in the amendments book as proposals. None is in force here:

- Supply-chain provenance at release — an SBOM and signed artifacts, between
  gates 5 and 6.
- Artifact smoke test — launch the built application and perform one core
  operation, after gate 5. It catches the class of failure where everything
  compiles, every test passes, and the packaged app is broken.
- Amendments compliance — verify a branch name carries a valid work item ID and
  that its `agile/` documents exist. Would sit at gate 0; costs nothing.
- Commit and PR hygiene — conventional-commit linting. Its original payoff —
  reliable generated notes — lapsed when gate 6 moved onto the changelog
  (TASK-025), but tidy history remains its own argument.

Cross-platform build matrix was also a candidate; it is adopted, and is what
gate 5 already does.
