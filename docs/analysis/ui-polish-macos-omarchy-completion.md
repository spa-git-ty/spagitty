<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Completion handoff: UI polish, macOS, and Follow Omarchy

Answers [`ui-polish-macos-omarchy-claude-handoff.md`](ui-polish-macos-omarchy-claude-handoff.md).
Baseline `e4ff3c4` (0.7.0). Seven work items, all merged to `main`.

The report asked for a handoff that "identifies what changed, includes
comparable screenshots and measured results, states the actual macOS signing
policy, and lists remaining limitations", and observed that passing unit tests
alone is not evidence the UI is polished or the DMG opens. Both of those remain
true, and the limitations section below is the important part of this document.

## The canonical amendments book was still missing

`/home/maxmya/dev/agents/docs/AMENDMENTS.md` does not exist on this machine.
`docs/AMENDMENTS.md` requires that this be reported rather than worked around,
so it is reported here as it was in the original report. The work followed the
repository's own observable rules — the `agile/` record gate enforced by
`tools/record.test.ts`, the comment style, the coverage floor, the changelog
requirement — and nothing was reconstructed from summaries.

## What changed

| Item | Stage | What |
| --- | --- | --- |
| [TASK-040](../../agile/items/TASK-040-the-macos-download-says-what-it-is.md) | 1 Trust | macOS signing policy, both architectures, artefact verification, honest notes, checksums |
| [TASK-039](../../agile/items/TASK-039-one-colour-system-in-every-dialog.md) | 2 Consistency | Eleven components off undefined tokens and off every literal colour; 83 type sizes tokenized |
| [BUG-030](../../agile/items/BUG-030-the-toolbar-offers-what-it-cannot-do.md) | 2 | Inert toolbar controls removed; Pull/Fetch alternatives made visible and keyboard-reachable; one shortcut formatter |
| [BUG-031](../../agile/items/BUG-031-the-window-opens-in-the-wrong-theme.md) | 2 | First-paint theme; source separated from resolved mode; live Follow system |
| [TASK-041](../../agile/items/TASK-041-the-frame-stops-competing-with-the-work.md) | 3 Composition | Chrome height, grouped destinations, adaptive widths, compact graph, reduced-motion fix |
| [TASK-042](../../agile/items/TASK-042-one-control-scale-and-a-platform-policy.md) | 3 | Last eight type sizes; hover motion; macOS window policy |
| [FEAT-080](../../agile/items/FEAT-080-follow-omarchy.md) | 4 Desktop fit | Follow Omarchy |

Each carries an item, a plan, an automated-test record and a manual sweep.
[TASK-043](../../agile/items/TASK-043-the-linux-renderer-and-the-accessibility-bridge.md)
is recorded as backlog for the one thing that needs hardware this machine does
not have.

## The macOS signing policy, stated

**Interim lanes** (`draft-release.yml`, `prerelease.yml`): Developer ID when a
certificate is configured, otherwise an **ad-hoc signature**
(`APPLE_SIGNING_IDENTITY=-`). **Production** (gate 5 on `main`): Developer ID,
notarized and stapled — and **a missing certificate fails the gate** rather than
publishing an unsigned build.

Three states, and the difference is the substance of the fix:

- *unsigned* — no signature. macOS says **damaged** and offers Move to Bin.
  There is no Open Anyway path from that dialog. **This is what every Spagitty
  macOS build was**, and it is the ranked explanation for the report's symptom.
- *ad-hoc signed* — a real seal over the app's own bytes with no identity.
  macOS says **unidentified developer**, which has a documented first-open path.
  It is not notarization and is not a promise of a warning-free install.
- *Developer ID signed and notarized* — the only policy whose acceptance
  criterion is that the app opens with no damaged-app warning at all. **It has
  never run**: there is no Apple Developer account for this project.

Also fixed: `macos-13` was retired and is gone; gate 5 and the alpha lane each
built one `macos-latest` job, which is Apple silicon, so **no published release
has ever carried an Intel Mac download**. Every lane now builds both. The
finished `.dmg` is mounted on the runner and asked three separate questions —
`hdiutil verify`, `codesign --verify --deep --strict`, `spctl --assess` — plus
`file` on the executable, because they answer different things and a valid
ad-hoc signature passes the second and fails the third by design. Releases carry
`SHA256SUMS`. The release notes no longer prescribe `xattr -d
com.apple.quarantine`; it survives in `docs/ci.md` as a diagnostic, labelled a
bypass.

**The author's own failing download has not been seen.**
[`TASK-040-sweep.md`](../../agile/testing/TASK-040-sweep.md) is that diagnosis,
written as a procedure with the evidence to record, and it is unrun.

## Measured results

### Follow Omarchy, on this machine

`sushi-dark-palette`, read by the Rust adapter from
`~/.local/state/omarchy/current/theme/colors.toml` and derived by the frontend:

```
bg #191724   panel #13101e   ink #fcfcfd   muted #86868a   accent #cf6348
onAccent #191724   danger #bda8a4   warn #cd9071   ok #8fa487
lanes #778291 #a1758a #bda8a4 #4a89a4 #8fa487
```

| Pair | Desktop's own | Derived |
| --- | --- | --- |
| foreground on background | 17.22:1 | 17.22 — unchanged |
| **muted on background** | **2.75:1 — fails** | **4.87:1 — passes** |
| label on accent | 3.70:1 with light text | 4.65:1, using the dark ground |
| each of five lanes on background | — | 4.53, 4.56, 7.82, 4.54, 6.58 |

The report's four numbers were recomputed independently by
`src/lib/colour.ts` and match. Two of the desktop's own roles fail contrast when
copied literally, and one of them — `muted` — is what every timestamp and path
in the application is painted with. That is why the palette is derived rather
than copied.

### Pixels returned to the work

| Change | Before | After |
| --- | --- | --- |
| Chrome above a screen header | 110px | 100px |
| Lane column, three-lane history (compact) | 149px | 72px |
| Rail default at 1280 | 186px | narrower, scaled to the window |
| Detail default at 1280 | 270px | narrower, scaled to the window |

### Suite

| | Before | After |
| --- | --- | --- |
| Frontend | 125 files / 2,611 tests | **131 files / 2,821 tests** |
| Rust | 74 | **95** |
| `bun run check` | 0 errors | 0 errors, 0 warnings over 1,152 files |
| Coverage | statements 79.8, **branches 71.26**, functions 77.94, lines 81.74 | all four above the 70% floor |

`cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all
--check` are clean. `bun run build` succeeds.

## Two defects that tests caught and reasoning did not

Worth recording, because both were reasoned about first and reasoned wrongly.

**`onAccent` came back `#000000`** rather than the desktop's own near-black,
because the first implementation took the *most* readable candidate. Contrast is
a threshold to pass, not a score to win; picking the maximum returns a colour
belonging to nobody's palette.

**Threading the dragged span into `laneNodeRadius`** silently reversed
FEAT-039's decision that a node's size follows the history's depth rather than
the column's width. Two assertions written long before this work failed
immediately.

A third is a test rather than code: `chrome.test.ts` asserted that Undo and Redo
carried `title="Not built yet"`. Two dead buttons shipped with a passing test
whose job was to check the apology was still attached.

## Remaining limitations

**No screenshots, and no before/after captures.** The review's stage-3 evidence —
identical size, theme, repository, data and selection, at 1280×800, 1440×900 and
a narrow tiled window, at 100/125/150% — needs a running Tauri window and a
person looking at it. This work changes the geometry those captures would be of;
it does not produce them. Every item's sweep says which captures it owes.

**Nothing has run on a Mac.** The signing policy, the artefact verification, the
Intel lane and the entire macOS window policy are unproven. `TASK-040-sweep.md`
and `TASK-042-sweep.md` tickets 5–9 are the first things to run.

**No CI run.** The three workflows are edited and not executed. YAML validity
was checked with a parser and the collection script was run against a fake
bundle tree; step order, `needs:` and composite-action `if:` are not executable
here.

**The production signing branch has never run.** No Apple Developer account
exists. Gate 5 now refuses to build macOS without one, which means `main`
currently publishes no macOS download at all where it previously published an
unsigned one. That is the deliberate half of the trade: the draft and alpha
lanes still produce a Mac download and now say truthfully what it is.

**Follow Omarchy has been proved on one desktop and one palette**, plus a light
palette and a bare one in tests. The live watch — `omarchy-theme-set` while the
window is open — is `SWEEP-004` and needs the app running.

**Compact graph has not been looked at.** Every number is reasoned and asserted;
none has been seen on a screen.

**`platform.rs` is untouched.** The review asks for profiling on representative
GPUs and an accessible path past the disabled at-spi bridge. There is one
machine here, and removing a known stability fix on appearance grounds is what
the review says not to do. TASK-043 records it rather than guessing.

**No performance traces.** The review asks for release-build traces before
choosing changes, a ~100ms interaction budget and 60Hz scrolling. None were
collected. Nothing here was chosen on a performance argument; the one
performance-adjacent claim — that the boot script adds no measurable cost — is a
sweep ticket, not a measurement.
