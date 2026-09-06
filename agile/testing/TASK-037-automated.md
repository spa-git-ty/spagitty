<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-037 — Automated tests

**Item:** [`agile/items/TASK-037-updates-that-actually-arrive.md`](../items/TASK-037-updates-that-actually-arrive.md)

## What was tested

`tools/updates.test.ts` — twelve tests over the build lanes and the update
source (eight cases, two of them run once per lane), beside `record.test.ts` in
the ordinary suite. Plus one addition to `crates/spagitty-core/src/update.rs`.

Both defects in this task were invisible to every existing test because neither
lived in code. The assertions are therefore joins between files that nothing
else reads together.

| Test | Asserts |
| --- | --- |
| *lane* stamps the release tag into it | Each of the three lanes sets `SPAGITTY_RELEASE` **on the step that builds the bundle** — not merely somewhere in the file. This is the defect that made the update check inert. |
| *lane* exports the AppImage update information | Each lane uses the composite action, so a fourth lane cannot be added without it. |
| collects the zsync alongside the AppImage | `release-assets` picks up `*.AppImage.zsync`. An AppImage attached without it advertises a URL that answers 404. |
| is a five-field gh-releases-zsync source | The shape `AppImageUpdate` and AppShelf both parse, and `latest` in the tag field. |
| names the project the in-app update check already reads | The AppImage's update source and `update.rs`'s endpoint are the same repository, in the same letters. |
| globs for the file the bundler actually writes | **The one this file exists for.** The glob is matched against `{productName}_{version}_amd64.AppImage.zsync`, composed from `src-tauri/tauri.conf.json` the way `tauri-bundler` composes it. Renaming the product now fails a test rather than the updater. |
| does not glob the AppImage itself | An updater reads whatever the glob matches as a zsync control file. Matching the bundle fails like a corrupt download. |
| does not glob another platform's download | The `.exe`, `.dmg` and `.deb` in the same release are excluded. |
| `the_endpoint_is_spelled_the_way_the_repository_is_spelled` | In `update.rs`. The endpoint is canonical lowercase, so the two update paths cannot disagree on spelling. |

**What would have to break for these to fail.** A lane that builds without
stamping the tag, a glob that no longer matches the bundle, an update source
pointed at a different repository, or a `.zsync` that stops being collected —
which is the complete list of ways this task can regress silently.

## Run against the broken state first

As the record for this repository requires of a new test, each of the two
load-bearing assertions was pointed at the defect it was written for.

**The tag stamp**, with gate 5 returned to what it was before this change — the
`SPAGITTY_RELEASE` line removed from the build step:

```
 × .github/workflows/gates.yml stamps the release tag into it
   Tests  1 failed | 11 passed (12)
```

One test fails, and it is the one written for the defect. The other two lanes
stay green, which is the point: `draft-release.yml` was always correct, and a
test that failed for all three would not have been describing the bug.

**The glob**, with the update information set to the `*x86_64.AppImage.zsync`
the brief originally asked for:

```
 × globs for the file the bundler actually writes
   AssertionError: expected false to be true
   Tests  1 failed | 11 passed (12)
```

This is the assertion that decided the `amd64` question. It is not an opinion
about naming: the glob it rejects is the one that would have matched no asset on
any release.

## Reproduced outside the suite, because the suite cannot run a workflow

The parts that are shell and `appimagetool` were exercised directly rather than
asserted about, since a test double for them would only prove the double
matched the file it was copied from.

A throwaway AppDir was bundled with the real tool and the real flag:

```
appimagetool Spagitty.AppDir \
  -u 'gh-releases-zsync|spa-git-ty|spagitty|latest|*_amd64.AppImage.zsync' \
  out/Spagitty_0.5.1_amd64.AppImage

zsyncmake is available and updateinformation is provided, hence generating zsync file

readelf -p .upd_info out/Spagitty_0.5.1_amd64.AppImage
String dump of section '.upd_info':
  [     0]  gh-releases-zsync|spa-git-ty|spagitty|latest|*_amd64.AppImage.zsync
```

That is the item's first acceptance criterion, met by the tool the lanes use.
The generated `.zsync` carries `Filename:` and `URL:` headers naming the
AppImage relatively, which is what makes renaming the pair fatal and is why
`release-assets` refuses to.

**One thing worth writing down:** `appimagetool` writes the `.zsync` into the
working directory, not beside the output file. `release-assets` searches the
tree rather than a directory, so it finds it either way — but a collector that
looked next to the AppImage would have found nothing.

The collection script was then extracted from `release-assets/action.yml` and
run verbatim against a fake bundle tree, in the passing case and in every
failure it is meant to catch:

| Case | Result |
| --- | --- |
| AppImage with update information and its zsync | Exit 0. Both collected. `… is updatable: gh-releases-zsync\|…` |
| AppImage built without `-u` | Exit 1. `has no gh-releases-zsync update information in .upd_info (found: '')` |
| Update information present, `.zsync` not collected | Exit 1. `…zsync is missing` |
| Pair renamed to `x86_64`, glob still `amd64` | Exit 1. `globs for '*_amd64.AppImage.zsync', which does not match …` |
| macOS suffix run over a tree holding both | Exit 0. The `.dmg` gains `-macos-arm64`; the AppImage pair is untouched. |

## What is not covered, and why

- **The workflows themselves.** Step order, `needs:` between the new `tag` job
  and the two that read it, and whether GitHub honours `if:` on a composite
  action step are not executable here. `SWEEP-001` to `SWEEP-004` carry them.
- **`linuxdeploy-plugin-appimage` honouring `$UPDATE_INFORMATION`.** Read in its
  source — `{"LDAI_UPDATE_INFORMATION", "UPDATE_INFORMATION", …}` becomes
  `-u` on the `appimagetool` argument list — and true of the version downloaded
  per build. Tauri falls back to an older built-in copy when that download
  fails, which is exactly why `release-assets` checks the built file instead of
  trusting the variable was read.
- **An actual in-place update.** That needs two published releases, the second
  carrying a `.zsync`. `SWEEP-005` and `SWEEP-006`.

## Coverage

`vite.config.ts` scopes the denominator to `src/lib/**` and `src/routes/**`, so
a tool in `tools/` moves it in neither direction. The Rust addition is one
assertion in an existing test module.
