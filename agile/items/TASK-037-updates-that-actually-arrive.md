<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-037 — Updates that actually arrive

**Status:** Done.
**Screen:** — (CI/CD, and the update check behind Settings → Behaviour, 1K).
**Raised by:** the author: "Add AppImage update capabilities (zsync /
gh-releases-zsync) to our AppImage build and release pipeline", and then "also
fix the inside app update check it's broken".

## Problem

FEAT-054 built a check that tells somebody a newer Spagitty exists. Two things
were wrong with the sentence it was trying to say, and both of them lived in the
build lanes rather than in any code the tests can see.

**It could not tell them what to do about it.** An AppImage is one file with no
package manager behind it, so nothing on the machine knows it can be replaced.
The answer the format has for this — an update source in the runtime's
`.upd_info` section, and a `.zsync` beside the release asset — was never
embedded, so no AppImage manager could act on the news. The only route was:
read the notice, open a browser, download ninety megabytes, replace the file by
hand.

**It could not tell them at all.** `SPAGITTY_RELEASE` is the only thing that
identifies a build — `Cargo.toml` says one version across a whole series of
releases, which is why the check compares tags and not version numbers. It was
set in `draft-release.yml` and **nowhere else**. Gate 5, which builds what a
`main` release publishes, and `prerelease.yml`, which builds every alpha, both
left it unset. So every build anybody has ever downloaded reported
`Channel::Development`, and a development build is never behind anything.

The feature shipped, its ten unit tests passed, and it has never once been able
to say yes. Nothing in `crates/spagitty-core/src/update.rs` is wrong, which is
exactly why nothing in that file caught it.

A third thing, smaller: the endpoint was spelled `Spa-git-ty/spagitty`. GitHub
resolves an owner login case-insensitively so it answered, but it is not a path
this project has, and the AppImage's embedded update source has to name the same
repository in the same letters.

## Change

- `.github/actions/appimage-update-info` — a composite action exporting one
  string, `gh-releases-zsync|spa-git-ty|spagitty|latest|*_amd64.AppImage.zsync`.
  Tauri has no configuration for this: its bundler shells out to `linuxdeploy`,
  whose `appimage` output plugin forwards `$UPDATE_INFORMATION` to
  `appimagetool -u`, which embeds the string and writes the `.zsync`. One action
  rather than three copies, for the reason `release-assets` exists at all.
- `.github/actions/release-assets` collects `*.AppImage.zsync`, never renames
  the pair when it disambiguates macOS filenames, and **refuses** an AppImage
  whose `.upd_info` is empty, whose `.zsync` is missing, or whose glob does not
  match the file just collected.
- All three lanes use the action on their Linux runner, and all three now bake
  `SPAGITTY_RELEASE` into the build.
- `prerelease.yml` gets a `tag` job, the shape `draft-release.yml` already has:
  the tag is worked out and checked for reuse **before** anything is built,
  because the build is stamped with it. The publish job reads that output
  instead of recomputing the same arithmetic.
- `crates/spagitty-core/src/update.rs` is spelled `spa-git-ty`, and its module
  documentation records what was inert and why the file could not have known.
- `tools/updates.test.ts` — the joins nothing else reads together.

## `amd64`, not `x86_64`

The brief asked for `*x86_64.AppImage.zsync`, which is the convention the
AppImage documentation uses. Tauri names the bundle
`Spagitty_<version>_amd64.AppImage`, and the last field of the update
information is a glob matched against the names of a release's assets, so
`x86_64` would have matched nothing. The alternatives were to rename the pair
after the build — which breaks it, because the `.zsync` names the AppImage it
patches and resolves it relative to its own download URL — or to say `amd64`.
The author chose `amd64`. `tools/updates.test.ts` matches the glob against the
name composed from `src-tauri/tauri.conf.json`, so this cannot drift quietly.

## Non-scope

- **A self-updater.** Spagitty still does not replace its own binary; FEAT-054
  declined that deliberately and this does not reverse it. What is added is the
  metadata a tool the *user* chose to run needs in order to do it.
- **`.deb` and `.rpm` update sources.** Those have package managers behind them
  and are a different problem.
- **Signing the AppImage.** `appimagetool -s` and a key in this repository are a
  separate decision, and it belongs with macOS and Windows signing rather than
  on its own.
- **Retro-fitting the released builds.** Amendment 14 forbids moving a tag, so
  every release up to `0.5.1` stays as it is. They carry no update information
  and cannot report themselves out of date; `0.6.0` is the first that can, and
  the first anybody can update *from* is the one after it.

## Acceptance criteria

- `readelf -p .upd_info` on a built AppImage prints a non-empty
  `gh-releases-zsync|…` string.
- The release carries both the `.AppImage` and the `.AppImage.zsync`, and the
  glob in the embedded string matches the name of the attached `.zsync`.
- A build produced by any of the three lanes reports `Channel::Released` and its
  own tag, not `Development`.
- `release-assets` fails, before anything is tagged, on an AppImage with no
  update information.
- The in-app check and the AppImage's update source name the same repository in
  the same letters.

## Dependencies

FEAT-054 built the check this makes work. TASK-025 built the release lane it is
added to, and `release-assets` carries the account of why a shared action exists.
