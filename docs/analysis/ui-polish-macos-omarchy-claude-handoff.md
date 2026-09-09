# Claude Code handoff: UI polish, macOS launch failure, and Omarchy appearance

Date: 2026-09-09. Source baseline: `e4ff3c4` (`0.7.0`).

The requested outcome is a Git client that feels composed, responsive, and trustworthy beside GitKraken, a macOS download with a verified installation path, and an additional appearance option that matches the active Omarchy desktop. This report is an implementation brief; the fixes described below have **not** been implemented.

## Evidence and limits

Reviewed the current Svelte/Tauri source, theme and layout tests, all three release workflows, release-asset collection, and the checked-in Graph, Diff, and Working copy screenshots under `docs/alpha/`. Those screenshots are historical: their labels and layout differ from current code. For example, the current sidebar already hides Open repository when a repository is open and has moved repository status into the bottom strip. Do not re-file those old defects.

The in-app browser was unavailable, so there was no live visual or performance verification. No failing DMG, affected macOS version, Mac architecture, exact dialog, or macOS diagnostic output was available. The macOS explanation is a ranked diagnosis, not a confirmed finding about the user's particular download.

The required canonical amendments book at `/home/maxmya/dev/agents/docs/AMENDMENTS.md` was missing. The repository's `docs/AMENDMENTS.md` is only a pointer and explicitly requires reporting this limitation. Claude must obtain and read the current canonical book, including Appendix A, before implementation; do not reconstruct it from comments or historical summaries.

## Why the UI can feel less finished

The main problem is inconsistent execution across the interface: some components use the design system, others use their own colors and sizes; the most prominent toolbar offers inert actions; and a lot of space and visual emphasis goes to navigation and graph decoration. These issues accumulate into a prototype impression even when the underlying Git functionality works.

GitKraken's useful reference is its organization around a persistent repository context, grouped actions, a graph, and a selection/details panel. Its documentation describes that organization and its controls. The recommendation here is to adopt that clarity while preserving Spagitty's identity and Farm workflow. This is a design assessment, not a measured head-to-head usability study. [GitKraken interface guide](https://help.gitkraken.com/gitkraken-desktop/interface/)

There is already useful groundwork: eight palette families, separate semantic status colors, shared layout metrics, a centralized SVG icon system, resizable panels, graph virtualization, device-pixel-aware canvas rendering, and extensive component tests. Build on these.

### Confirmed source findings and their consequences

| Priority | Finding | Why it affects perceived quality | Concrete change |
| --- | --- | --- | --- |
| P1 | `src/lib/ui/flat.test.ts` maintains a `KNOWN` list of **11 components** using undefined CSS tokens. `WorktreesModal.svelte`, for example, reads `--fg`, `--dim`, `--bg-2`, and `--border-soft` with fixed fallbacks. | A polished main screen can open a dialog from a visibly different color system. Light and imported themes expose this particularly clearly. | Migrate these components to the actual semantic tokens. Remove each exception as it is fixed; end with an empty exception list. Verify rendered states as well as token names. |
| P1 | `Toolbar.svelte` puts Undo and Redo in `GROUPS` with only `title: 'Not built yet'`. Rendered buttons have neither a real action nor `disabled`. | Clicking a prominent control does nothing. A tooltip does not repair that loss of trust. | Remove unfinished actions from the normal toolbar or visibly disable them with an accessible explanation. Only add functional undo through a separately designed, operation-specific recovery model. |
| P1 | `src/app.html` starts with `data-theme="light"`; `theme.init()` runs in layout `onMount`. `theme.init()` samples the system preference once and persists it as an explicit mode. | A saved dark/custom theme is applied after the initial light markup; a flash is possible. There is no continuing Follow system behavior. | Apply a cached validated palette before showing the workspace, with a CSP-compatible boot strategy. Store preference separately from resolved light/dark mode and listen for system changes when appropriate. Measure cold launch. |
| P1 | `metrics.ts` reserves 30px title bar + 30px tabs + 50px toolbar before the screen header. Fourteen sidebar destinations have broadly equal treatment. | The application frame competes with the work. Occasional tools, routine Git work, and Farm supervision are difficult to distinguish at a glance. | Simplify the chrome and group destinations; preserve Farm prominence. Provide contextual access to occasional tasks without removing keyboard or navigation access. |
| P1 | At default metrics, sidebar 186px + refs 186px + minimum lanes 149px + detail 270px consumes about 791px before dividers and message metadata. Five lane columns are reserved even for a simple history. | At 1280px, opening detail leaves roughly 489px for messages and their metadata. Simple histories also waste graph width. | Use adaptive initial widths, persistent user resizing, and a compact graph mode. Give the commit subject first claim on space. Preserve one shared geometry model across refs, rows, and canvas. |
| P2 | `TitleBar.svelte` draws the same three neutral window controls on the right on every platform; Tauri disables decorations and enables transparency globally. | On macOS this does not match familiar window controls and can feel like a web page inside a custom frame. | Add a deliberate platform policy. Prefer native macOS controls/title-bar integration; verify title-bar drag behavior, full screen, focus, menus, and shortcuts on a real Mac. Keep Linux tiling behavior separately tested. |
| P2 | Default UI text is 15.6px and secondary text 13.2px, but title-bar controls and several dialogs still use fixed 10–13px sizes. `Btn.svelte` and `NavRail.svelte` move controls on hover/press. | Typography and target sizes change by screen, while incidental motion makes dense work feel less steady. | Finish tokenizing typography and control sizing. Prefer stable targets with color/focus changes; reserve motion for meaningful state changes. |
| P2 | Graph nodes use generated multicolor marble portraits, 22px nominal diameter and 26px lane pitch. Historical screenshots show repeated portraits forming a visually dominant column. | Decoration can be louder than ancestry, selection, and commit subjects. This is a design judgment, not a graph correctness defect. | Offer compact graph nodes and quieter identity fallbacks; put larger portraits in author/detail contexts. Retain the current avatar option and offline behavior. |
| P2 | `src-tauri/src/platform.rs` defaults to disabling the DMABuf renderer on Wayland and sets `NO_AT_BRIDGE=1` unless overridden. | Rendering workarounds and the disabled accessibility bridge make native testing essential. Extra blur may be costly; assistive technology may lose access. | Profile the actual release build on representative GPUs. Investigate a version/hardware-scoped workaround and an accessible path; do not remove known stability fixes on appearance grounds. |

Several source comments describe earlier glow/glass treatments although current declarations set `animation: none` or `box-shadow: none`. Inspect declarations and runtime output rather than treating comments as current visual evidence. The brand document's flat-design mandate applies to the **mark**; it is not a blanket ban on useful surface separation throughout the app.

### The polish pass to implement

Establish one calm workspace hierarchy: repository identity and current branch; navigation; the active work surface; selection details. Use restrained neutral surfaces, one primary action per task area, clear selection, and status colors with consistent meanings. Additions, deletions, warnings, active navigation, and branch identity must remain distinguishable.

Use these as initial design targets, then validate against rendered screens:

- A shared 4/8px spacing rhythm, roughly 28–32px compact controls/rows, consistent icon sizes and stroke weights, and a small radius scale separating controls from dialogs. Reconcile changes with text scaling instead of forcing every row to a fixed height.
- Readable 13–14px UI text and 14–15px code as a starting compact preset, preserving larger-text preferences. Use the system UI font; reserve monospace for code, SHAs, and suitable metadata. Do not shrink text simply to hide layout problems.
- Clear differences among normal, hover, selected, keyboard focus, disabled, pending, and failed states. Loading must retain context and avoid jumping layouts. Errors should explain the failed action and offer an appropriate retry/recovery action.
- Finish shared button, field, menu, dialog, table-row, and empty-state primitives before polishing individual screens. Extend `Btn` with explicit size/state semantics where useful; avoid another parallel button system.
- Make Fetch/Pull alternatives discoverable with a visible dropdown and keyboard access; they currently rely on right-click handlers. Label shortcuts as Cmd on macOS and Ctrl on Linux/Windows using a shared formatter.
- In Graph, make the selected commit unmistakable without repainting every unrelated row loudly. Keep node centers and rows aligned at every zoom. Verify branch overflow, long subjects, author columns, and details together.
- In Working copy, make staged versus unstaged work and the commit action clear together. A clean repository should provide a concise success state with useful navigation, rather than merely looking empty. Test the dirty state, not only clean screenshots.
- In Diff/Conflicts, prioritize code legibility, line numbers, reliable synchronized split scrolling, hunk controls, and long paths. Avoid nesting several bordered cards around one code surface.
- In Farm, make task status, active work, failure, and the next action easy to scan. Keep animation and reward effects out of the critical reading path and honor existing preferences.

Preserve existing focus management and reduced-motion support. Check Svelte transitions too, including the unconditional `fly` transition in `+layout.svelte`; a global CSS media query is not sufficient evidence that every runtime transition honors reduced motion.

Require at least 4.5:1 contrast for ordinary readable text and 3:1 for essential non-text control/state indicators, with the relevant WCAG exceptions. Measure final composited colors, including hover, selection, and translucent surfaces, not just raw palette entries. [WCAG 2.2](https://www.w3.org/TR/WCAG22/)

## macOS: why “damaged” instead of a permission prompt?

Unsigned does **not** guarantee a friendly Open Anyway path. The dialog depends on the app's actual signing/integrity state and macOS policy. Tauri explicitly documents the broken-app warning for browser downloads, supports ad-hoc signing, and notes that ad-hoc builds still require a security exception. Apple separately distinguishes an unidentified developer from an app detected as modified or damaged. The wording alone cannot prove physical download corruption. [Tauri signing](https://v2.tauri.app/distribute/sign/macos/), [Apple's warning descriptions](https://support.apple.com/en-us/102445)

First determine whether the error names **the `.dmg` before it mounts** or **`Spagitty.app` when launched**. A DMG is a container; an app signature and a disk-image integrity check answer different questions.

### What this repository establishes

- `src-tauri/tauri.conf.json` has no explicit macOS signing identity or notarization configuration.
- `draft-release.yml` explicitly describes unsigned distribution. Its release notes promise right-click Open or `xattr -d com.apple.quarantine /Applications/Spagitty.app` as the remedy.
- `draft-release.yml`, `prerelease.yml`, and the release build in `gates.yml` do not configure Apple signing/notarization credentials or run a final macOS integrity/launch gate.
- `.github/actions/release-assets/action.yml` checks asset collection and Linux update metadata. It does not verify DMG integrity, the contained app signature, or first launch after downloading.
- Only the draft workflow explicitly separates ARM64 and Intel builds. Its Intel job still uses `macos-13`, a retired hosted image. Update the matrix consistently across release lanes using supported explicit architectures; the obsolete runner is an additional build defect, not an explanation for a particular already-downloaded app being called damaged. [GitHub retirement notice](https://github.blog/changelog/2025-09-19-github-actions-macos-13-runner-image-is-closing-down/), [current runner labels](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job)

Most plausible: a downloaded, quarantined app without a properly configured distribution-signing path. Other possibilities needing evidence: an absent/invalid ad-hoc signature, changed bundle resources after signing, incomplete download, or malformed DMG. Wrong architecture/minimum OS must also be checked, although these often produce different errors. Do not claim the exact root cause until inspecting the failing asset.

### Diagnostic recipe for Claude on a Mac

Record release URL/tag, asset basename, SHA-256, macOS version, architecture, the exact warning, and which double-click produced it. Download the **published** asset using a browser on a clean user/VM so quarantine behaves like a user's download. Do not clear quarantine or re-sign before collecting evidence.

Example read-only diagnostics; replace the filenames with the actual ones:

```sh
sw_vers
uname -m
shasum -a 256 ~/Downloads/Spagitty.dmg
hdiutil verify ~/Downloads/Spagitty.dmg
xattr -l ~/Downloads/Spagitty.dmg
codesign -dvvv /Applications/Spagitty.app
codesign --verify --deep --strict --verbose=2 /Applications/Spagitty.app
spctl --assess --type execute --verbose=4 /Applications/Spagitty.app
xattr -lr /Applications/Spagitty.app
file /Applications/Spagitty.app/Contents/MacOS/spagitty
plutil -p /Applications/Spagitty.app/Contents/Info.plist
```

Inspect the app inside the mounted DMG as well as the installed copy. Obtain the executable name from `CFBundleExecutable` if it differs. On supported macOS versions, include `syspolicy_check distribution` and record its output. For a notarized release, additionally run `xcrun stapler validate` on the artifacts expected to carry tickets.

Interpret results separately: `hdiutil` tests the container; `codesign --verify` tests signature integrity; `spctl` assesses policy/trust. A valid ad-hoc signature can pass integrity and still fail Gatekeeper's default trust assessment. CLI checks are not a substitute for the final Finder launch test. Apple documents strict signature verification and its limitations. [Apple code-signing verification](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/Procedures/Procedures.html), [notarization troubleshooting](https://developer.apple.com/documentation/security/resolving-common-notarization-issues)

### Two explicit release policies

**Interim, without an Apple Developer identity:** configure Tauri's macOS bundle signing identity as `"-"` to create an explicit ad-hoc signature. Verify the finished app inside the finished DMG. This is worth trying and testing, but it is not notarization and must not be marketed as guaranteed warning-free installation. Document the exact tested Privacy & Security → Open Anyway flow for supported OS versions. If a version still presents “damaged,” keep investigating and report that limitation. [Tauri ad-hoc signing](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing)

**Production distribution:** Developer ID Application signing, appropriate hardened-runtime/entitlement configuration, notarization, and stapling. Configure credentials in CI secrets, share the release policy across all three workflows, and fail the intended production lane if credentials or validation are missing. Do not silently downgrade a production release to an unsigned one. Verify the exact Tauri version's handling of the app and DMG; do not assume that one notarized artifact covers every item uploaded. [Tauri signing and notarization configuration](https://v2.tauri.app/distribute/sign/macos/)

Finish executable/resource changes before signing. Avoid “repairing” the packaged app with blanket deep re-signing after the fact. Publish checksums for the final artifacts and test a browser download of those exact bytes on each supported architecture. `--deep` above is for verification, not a proposed signing strategy.

Replace the current release-note promise of a universal one-step fix. Quarantine removal is a troubleshooting bypass, not proof that packaging is correct. Do not disable Gatekeeper globally or use quarantine stripping in the acceptance test. The normal production acceptance criterion is that the downloaded DMG mounts and the app launches through the expected first-open confirmation without a damaged-app warning.

## Additional appearance option: Follow Omarchy

Implement **Settings → Appearance → Follow Omarchy**, available when Omarchy is detected on Linux. This should read the **active desktop palette**, not hard-code Tokyo Night or assume Omarchy has one universal color scheme. Preserve all existing theme families and explicit user selections.

This machine provides direct evidence:

- Active theme name: `sushi-dark-palette`.
- Active palette: `/home/maxmya/.local/state/omarchy/current/theme/colors.toml`.
- Theme name file: `/home/maxmya/.local/state/omarchy/current/theme.name`.
- The older `~/.config/omarchy/current` directory does not exist here.
- Installed `/usr/share/omarchy/bin/omarchy-theme-set` builds a `next-theme` directory and replaces `current/theme`. Watching only the old palette file/inode will miss later replacements.

The upstream Quattro theming document also describes the state-directory layout. Treat detection as version-aware and capability-based. [Omarchy theme architecture](https://github.com/omacom/omarchy/blob/quattro/docs/theming.md)

### Integration design

1. In a small Linux backend adapter, probe the active state layout and legacy config layout. Respect XDG state/config locations where the installed version does; also support the actual default paths used by its scripts. Do not treat `XDG_CURRENT_DESKTOP=Hyprland` as sufficient proof of Omarchy. Use Omarchy-specific installation/state evidence plus a usable palette.
2. Read bounded TOML data in Rust and return a typed, validated palette/capability response through Tauri. Do not expose arbitrary filesystem reads to the frontend, import theme CSS, or execute/source theme files. Reuse an existing compatible parser if available; explain any new dependency in the handoff.
3. Separate theme **source preference** (`manual`, `system`, `omarchy`) from resolved palette/mode. Migrate existing family/mode storage without overriding the user's choice. Offer Follow Omarchy when detected; choosing it is persistent, and choosing a manual family opts out. System mode follows OS light/dark while retaining a selected built-in family.
4. Read semantic fields such as `background`, `foreground`, `accent`, selection colors, and status hues; use ANSI slots as compatibility fallbacks. Derive missing panel/hover/border colors consistently. Infer light/dark only if `mode` is absent.
5. Watch a stable parent directory, re-read after directory/symlink replacement, debounce bursts, and retain the last valid palette during partial updates. Recheck on application focus as a recovery path. Dispose listeners when disabled or the app closes. Avoid full Git graph reloads for a theme change.
6. Add a reactive **palette revision**, not just family/mode identity. `LaneCanvas.svelte` currently invalidates colors and portrait caches from `theme.id`; multiple imported dark palettes under the same `omarchy-dark` ID would otherwise risk stale graph colors.
7. Cache the last validated resolved palette for first paint. If files disappear, retain that palette for the session and expose a quiet status in Appearance; on a first launch without usable data, use the normal built-in fallback. Other platforms must work without Omarchy files or errors.
8. Do not edit the user's desktop palette, install desktop hooks, or force the app's fonts into a terminal monospace style. The requested feature is for Spagitty to follow the desktop. Handle Omarchy's tiled/flush windows through the existing window policy rather than adding another visible frame.

### Concrete palette preview from this desktop

These values are an observed example, not a universal Omarchy preset:

| App role | Observed source | Value / mapping |
| --- | --- | --- |
| Workspace | `background` | `#191724` |
| Chrome | `dark_bg` | `#13101e` |
| Raised surface | `lighter_bg` | `#2a2735` |
| Main text | `foreground` | `#fcfcfd` |
| Accent | `accent` | `#cf6348` |
| Secondary text | derive from foreground/background | Do not copy `muted = #5f5e63` directly for readable labels |
| Text on accent | choose by contrast | `#191724` is preferable to `#fcfcfd` for this accent |
| Graph/status | semantic hues, then ANSI fallbacks | Validate against actual surfaces and retain text/icon status cues |

Calculated from the local palette using sRGB relative luminance: foreground/background **17.22:1**; raw muted/background **2.75:1**; light foreground/accent **3.70:1**; dark background/accent **4.65:1**. Thus a literal palette copy would fail ordinary-text contrast in two common places. Preserve the desktop's character while adjusting derived app roles for readability; test against raised surfaces too.

## Delivery sequence and acceptance

| Stage | Deliverable | Required evidence |
| --- | --- | --- |
| 1 — Trust | Diagnose the exact macOS asset; repair release policy/matrix and misleading installation instructions. | Container/signature/policy results and a clean-machine browser-download launch recording for each supported architecture; clearly label interim ad-hoc versus production notarized results. |
| 2 — Consistency | Fix all 11 token exceptions, inert toolbar controls, shared sizing/states, and startup preference handling. | No undefined-token exceptions; coherent light/dark dialogs; truthful button behavior; keyboard and startup recordings. |
| 3 — Composition | Polish Graph, Working copy, Diff, then Farm and auxiliary screens using the shared system. | Before/after captures at identical size, theme, repository, data, and selection. Include narrow and high-density layouts. |
| 4 — Desktop fit | Add Follow Omarchy and platform-specific chrome behavior. | Live theme changes repaint DOM and canvas; local example and a light desktop palette both pass; manual override persists; non-Omarchy platforms retain current behavior. |

Suggested review matrix: 1280×800, 1440×900, and a narrow ~900px tiled window; 100%, 125%, and 150% scaling; a Retina Mac; small and complex histories; long branches/paths; clean and dirty worktrees; loading/failure/empty states; light/dark and imported palettes. Capture the actual Tauri webviews on macOS and Omarchy. Browser previews and happy-dom tests do not establish native rendering quality.

For performance, collect release-build traces before choosing changes. Target prompt visual feedback (roughly within 100ms for local interaction) and smooth 60Hz scrolling on the stated reference machine; record actual frame times and stalls. These are proposed budgets, not measurements of current performance. Do not attribute slowness to Tauri or rewrite the framework without evidence.

Add meaningful tests for palette parsing/fallbacks, old/new Omarchy layouts, directory replacement, preference migration, canvas revision, contrast on final surfaces, and final macOS artifacts. Keep existing graph geometry, focus, keyboard, and Git behavior coverage. Resolve canonical workflow/review requirements before implementation, run applicable tests and builds, and explain every added dependency. Do not turn this request into a new feature expansion or a logo redesign.

## Validation performed for this report

- `bun run check`: passed, zero errors and warnings.
- `bun run build`: passed; static frontend output generated.
- `bun run test`: **125 files / 2,611 tests passed** on the rerun outside the sandbox. The first run passed 2,606 tests and hit `spawnSync ... EPERM` in five development-style tests; those all passed on rerun. The rerun emitted Node experimental localStorage warnings.
- No Rust source or runtime configuration changed; Rust/native builds, macOS launch tests, and live visual QA were not run.
- Only this report was added. No dependencies added, no desktop settings changed, and no release published.

Claude's completion handoff should identify what changed, include comparable screenshots and measured results, state the actual macOS signing policy, and list remaining limitations. Passing unit tests alone is not evidence that the UI is polished or the DMG opens.
