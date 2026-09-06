<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-037 — Manual sweep

**Item:** [`agile/items/TASK-037-updates-that-actually-arrive.md`](../items/TASK-037-updates-that-actually-arrive.md)

This sweep is run at a terminal, on GitHub, and on a Linux desktop. Only
`SWEEP-007` happens inside Spagitty.

**Two tickets decide whether this task worked**, and neither can be run before a
release exists. `SWEEP-002` is the first build that carries update information;
`SWEEP-006` is the first that can be updated *onto* — it needs a second release
after this one, because there is nothing yet to update from. Until both are
filled in, this task is reviewed rather than verified.

| Ticket | Preconditions | Steps | Expected result | Priority | Pass/Fail |
| --- | --- | --- | --- | --- | --- |
| SWEEP-001 | A Linux machine with `bun`, the Rust toolchain and the Tauri build dependencies | `UPDATE_INFORMATION='gh-releases-zsync\|spa-git-ty\|spagitty\|latest\|*_amd64.AppImage.zsync' SPAGITTY_RELEASE=v0.6.0 bun run tauri -- build --bundles appimage` | It bundles. `target/release/bundle/appimage/` holds `Spagitty_0.6.0_amd64.AppImage`, and a `Spagitty_0.6.0_amd64.AppImage.zsync` exists somewhere in the tree — `appimagetool` writes it into the working directory, not beside the bundle. | High | |
| SWEEP-002 | A build from SWEEP-001, or the artifact from any lane | `readelf -p .upd_info <the .AppImage>` | It prints a **non-empty** `gh-releases-zsync\|spa-git-ty\|spagitty\|latest\|*_amd64.AppImage.zsync`. An empty dump, or `Section '.upd_info' … does not exist`, is the failure this whole task is about. | High | |
| SWEEP-003 | A branch under `draft/**` pushed to the remote | Wait for the draft workflow, then read the draft it leaves | The Linux assets are **two** files: the `.AppImage` and the `.AppImage.zsync`. The zsync's name is exactly the AppImage's plus `.zsync` — neither has picked up an architecture suffix. | High | |
| SWEEP-004 | The same run as SWEEP-003 | Open the `Collect the release assets` step on the Linux job | It ends with `… is updatable: gh-releases-zsync\|…`. If it instead errors about missing update information, the composite action did not run — check the `if:` on it. | High | |
| SWEEP-005 | A published release from gate 6, and its `.AppImage` downloaded and run once | In Spagitty, open Settings → Behaviour and press the update check | It reports the tag this build was cut as, **not** a development build. This is the half of the task with no test behind it: a `Development` channel here means `SPAGITTY_RELEASE` still is not reaching the binary. | High | |
| SWEEP-006 | Two published releases, both carrying a `.zsync`, with the older one installed | `AppImageUpdate <the installed .AppImage>` | It finds the newer release, transfers a fraction of the file rather than all of it, and leaves a working AppImage. | High | |
| SWEEP-007 | AppShelf installed, and the older AppImage from SWEEP-006 imported into it | Select Spagitty and press `Ctrl+U` | AppShelf reports an update rather than `No embedded update information (.upd_info)` or `Unsupported update protocol`. Note that AppShelf downloads the whole AppImage; it reads the zsync for its checksum, not for a delta. | Medium | |
| SWEEP-008 | A published release from before this change, e.g. `v0.5.1` | `readelf -p .upd_info` on its `.AppImage`, and `Ctrl+U` on it in AppShelf | No section, and AppShelf says so plainly. **This is correct.** Amendment 14 forbids moving a tag, so the old releases stay as they are; the ticket exists so that "the old one cannot update" is a known fact rather than a bug report. | Low | |
| SWEEP-009 | A `prerelease` run using an alpha number that has already been published | Run the workflow with that number | It fails in the `work out the tag` job, **before** any platform is built. Previously this failed after three builds had completed. No tag, no release. | Medium | |

## Negative paths this sweep deliberately covers

- **SWEEP-008** is the honest one. Nothing here retro-fits a published build,
  and anybody testing this will reach for the version they already have, which
  is the one version that cannot work.
- **SWEEP-009** proves the reordering in `prerelease.yml` rather than just the
  new variable: the tag is decided before the build because the build is stamped
  with it, and the cheap refusal moved ahead of the expensive work as a
  consequence.
- **SWEEP-005** is the only place the `SPAGITTY_RELEASE` fix is visible to a
  person. `tools/updates.test.ts` can see that the workflow sets the variable;
  only a downloaded build can show that it arrived.
