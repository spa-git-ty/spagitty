// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The desktop's palette, turned into one of Spagitty's (FEAT-080).
 *
 * `src-tauri/src/desktop.rs` finds and validates what Omarchy publishes and
 * hands back the desktop's own vocabulary — background, foreground, accent, the
 * status hues, sixteen ANSI slots. This file turns that into the fourteen
 * tokens every screen reads, and it lives beside `themes.ts` rather than in
 * Rust because what it does is **design**, with contrast arithmetic in it, and
 * design that the frontend cannot check is design nobody checks.
 *
 * # A literal copy would be unreadable, and this is measurable
 *
 * Calculated from the palette on the machine this was written against
 * (`sushi-dark-palette`), using sRGB relative luminance:
 *
 * | Pair | Ratio | Verdict |
 * |------|-------|---------|
 * | `foreground` on `background` | **17.22:1** | excellent |
 * | `muted` on `background` | **2.75:1** | fails ordinary text (4.5:1) |
 * | light `foreground` on `accent` | **3.70:1** | fails text on a filled button |
 * | dark `background` on `accent` | **4.65:1** | passes — so this is the label colour |
 *
 * So two of the roles a terminal palette carries are wrong for an application
 * the moment they are copied across, and one of them — `muted` — is what every
 * secondary label in Spagitty is painted with. A terminal's `muted` is a
 * comment colour in a monospace wall of text; a UI's `muted` is a timestamp
 * beside a commit subject. They are not the same requirement.
 *
 * The rule this file follows: **take the desktop's hues, derive the app's
 * roles.** `muted` keeps the desktop's colour but is walked toward the
 * foreground until it clears 4.5:1; `onAccent` is chosen by measurement between
 * the palette's own two extremes rather than assumed to be white. Everything
 * the desktop does not name is derived from what it does, at the same fixed
 * opacities `themes.ts` already uses for `line`, `soft`, `stripe` and
 * `placeholder` — so a followed palette is internally consistent in the same
 * way a built-in one is.
 *
 * # The lanes
 *
 * Five graph colours, and they have to be tellable apart from each other and
 * readable on the graph's own column. Semantic names first — `blue`, `magenta`,
 * `red`, `cyan`, `green`, which is the order the built-in families use — and
 * the matching ANSI slots as fallbacks, because a generator may name one and
 * not the other. Any that are still missing fall back to the default family's
 * lane for that position rather than being dropped: a graph with three lane
 * colours draws two branches in the same colour, which is worse than a graph
 * with two colours from somewhere else.
 */

import {
	contrast,
	firstReadable,
	hex,
	isDark,
	mix,
	parse,
	rgba,
	untilReadable
} from './colour';
import { DEFAULT_FAMILY, paletteOf, type Mode, type Palette } from './themes';

/** The desktop's own vocabulary, as `desktop.rs` returns it. */
export interface DesktopPalette {
	mode: string | null;
	background: string;
	foreground: string;
	accent: string | null;
	darkBg: string | null;
	lighterBg: string | null;
	muted: string | null;
	selectionBackground: string | null;
	selectionForeground: string | null;
	red: string | null;
	green: string | null;
	yellow: string | null;
	blue: string | null;
	magenta: string | null;
	cyan: string | null;
	/** Slots 0-15, positional: a gap stays a gap. */
	ansi: (string | null)[];
}

export interface DesktopTheme {
	available: boolean;
	name: string | null;
	layout: 'state' | 'legacy' | null;
	palette: DesktopPalette | null;
	reason: string | null;
}

/**
 * WCAG's two thresholds, named so the call sites read as requirements.
 *
 * 4.5:1 is ordinary text. 3:1 is an essential non-text indicator — a border
 * that means "selected", a status dot — and is what the built-in families hold
 * their accent and status colours to.
 */
const TEXT_RATIO = 4.5;
const INDICATOR_RATIO = 3;

/**
 * The alphas `themes.ts` derives its neutrals at, repeated here on purpose.
 *
 * They are the same numbers, and they are written twice rather than exported
 * from one place, because these two files answer different questions: those
 * were chosen once per family by a person looking at the result, and these have
 * to hold for a palette nobody has looked at. If a built-in family ever wants a
 * different `line` alpha, it should be able to have one without changing what a
 * followed desktop looks like.
 */
const ALPHA = {
	line: 0.26,
	softLight: 0.12,
	softDark: 0.11,
	placeholderLight: 0.3,
	placeholderDark: 0.26,
	stripe: 0.05,
	selection: 0.16
};

/** The first candidate that parses, or null. */
function first(...candidates: (string | null | undefined)[]) {
	for (const candidate of candidates) {
		if (!candidate) continue;
		const rgb = parse(candidate);
		if (rgb) return rgb;
	}
	return null;
}

/**
 * Light or dark, from the file where it says so and from the background where
 * it does not.
 *
 * The generated file on this machine carries `mode = "dark"` and there is no
 * reason to second-guess it. Where it is absent, the background's own
 * luminance is the only honest evidence — and it is the same question
 * `isDark()` answers for every other surface here.
 */
export function modeOf(palette: DesktopPalette): Mode {
	if (palette.mode === 'light' || palette.mode === 'dark') return palette.mode;
	const background = parse(palette.background);
	return background && isDark(background) ? 'dark' : 'light';
}

/**
 * Build a Spagitty palette from a desktop one.
 *
 * Total: every field of `Palette` is set from something. A palette that left a
 * token out would fall through to whatever the previous theme had, which is
 * how half a theme ends up on screen — the reason `themes.ts` makes every field
 * required in the first place.
 */
export function paletteFrom(desktop: DesktopPalette): Palette {
	const mode = modeOf(desktop);
	const fallback = paletteOf(DEFAULT_FAMILY, mode);

	const bg = parse(desktop.background) ?? parse(fallback.bg)!;
	const ink = parse(desktop.foreground) ?? parse(fallback.ink)!;
	const dark = mode === 'dark';

	/*
	 * Chrome and raised surfaces.
	 *
	 * The desktop names both — `dark_bg` and `lighter_bg` — and where it does
	 * not, they are mixed from the ground toward the ink by the small amount
	 * `app.css` uses for its own surface stack. `--panel` is chrome, which sits
	 * *away* from the viewer, so on a dark theme it darkens and on a light one
	 * it darkens too: a light theme's panel is `#e6e9ef` against a `#eff1f5`
	 * ground, which is a step down, not up.
	 */
	const panel =
		first(desktop.darkBg) ?? mix(dark ? { r: 0, g: 0, b: 0 } : ink, bg, dark ? 0.35 : 0.06);

	/*
	 * Secondary text.
	 *
	 * **The one role that is measured rather than copied.** The desktop's own
	 * `muted` is a terminal comment colour and fails ordinary-text contrast on
	 * every palette measured so far; it is used as the *starting hue* and walked
	 * toward the foreground until it is readable. Where the desktop names none,
	 * the foreground at the built-in families' own opacity is the answer, which
	 * is exactly what those families do.
	 */
	const mutedSource = first(desktop.muted) ?? mix(ink, bg, 0.66);
	const muted = untilReadable(mutedSource, ink, bg, TEXT_RATIO);

	/*
	 * The accent.
	 *
	 * Held to 3:1 against the ground, because it is a border, a link and a
	 * filled button — the same bar `themes.ts` holds every family's accent to,
	 * and the reason the light variants there are darker than their published
	 * swatches. A desktop accent that fails it is walked toward the foreground
	 * until it clears, which keeps the hue and moves the lightness.
	 */
	const accentSource = first(desktop.accent, desktop.selectionBackground) ?? parse(fallback.accent)!;
	const accent = untilReadable(accentSource, ink, bg, INDICATOR_RATIO);

	/*
	 * The label on a filled accent.
	 *
	 * Measured, not assumed. On the palette this was written against, the light
	 * foreground gives 3.70:1 on the accent and the dark background gives
	 * 4.65:1 — so white would have been the wrong answer, and it is the answer
	 * anybody would have written down. The desktop's own
	 * `selection_foreground` is offered first, since a palette that names one
	 * has already thought about it.
	 */
	const onAccent = firstReadable(
		accent,
		[first(desktop.selectionForeground) ?? bg, bg, ink, { r: 255, g: 255, b: 255 }, { r: 0, g: 0, b: 0 }],
		TEXT_RATIO
	);

	/*
	 * Status colours.
	 *
	 * Semantic name, then the ANSI slot that means the same thing, then the
	 * built-in family's — and each is held to the 3:1 the built-ins hold
	 * theirs to, because a "failed" that cannot be told from a "succeeded" is
	 * not a status colour.
	 */
	const status = (named: string | null, slot: number, bright: number, fallbackValue: string) =>
		untilReadable(
			first(named, desktop.ansi[slot], desktop.ansi[bright]) ?? parse(fallbackValue)!,
			ink,
			bg,
			INDICATOR_RATIO
		);

	const danger = status(desktop.red, 1, 9, fallback.danger);
	const warn = status(desktop.yellow, 3, 11, fallback.warn);
	const ok = status(desktop.green, 2, 10, fallback.ok);

	/*
	 * The lanes: blue, magenta, red, cyan, green, which is the order the
	 * built-in families use so a repository looks broadly the same after a
	 * theme change. Each held to 3:1 on the ground for the same reason the
	 * built-ins are: a lane you cannot see is a branch you cannot follow.
	 */
	const laneSources: [string | null, number, number, string][] = [
		[desktop.blue, 4, 12, fallback.lanes[0]],
		[desktop.magenta, 5, 13, fallback.lanes[1]],
		[desktop.red, 1, 9, fallback.lanes[2]],
		[desktop.cyan, 6, 14, fallback.lanes[3]],
		[desktop.green, 2, 10, fallback.lanes[4]]
	];

	/*
	 * Five lanes that are five colours.
	 *
	 * A desktop palette is free to repeat itself, and real ones do — Rosé Pine
	 * Dawn gives `blue` and `cyan` the same `#56949f`. Copied straight across,
	 * two branches in the graph would be drawn identically and there would be
	 * nothing on screen to say which was which. So each lane takes the first of
	 * its candidates that is readable *and* not already used, falling back to
	 * the built-in family's lane for that position, and finally to that lane
	 * nudged toward the ink — which is a colour from somewhere else, and still
	 * better than two branches that look like one.
	 */
	const used = new Set<string>();
	const lanes = laneSources.map(([named, slot, bright, fallbackValue]) => {
		const candidates = [
			first(named),
			first(desktop.ansi[slot]),
			first(desktop.ansi[bright]),
			parse(fallbackValue)!
		].filter((candidate) => candidate !== null);

		for (const candidate of candidates) {
			const value = hex(untilReadable(candidate, ink, bg, INDICATOR_RATIO));
			if (!used.has(value)) {
				used.add(value);
				return value;
			}
		}

		const nudged = hex(
			untilReadable(mix(ink, parse(fallbackValue)!, 0.2), ink, bg, INDICATOR_RATIO)
		);
		used.add(nudged);
		return nudged;
	}) as [string, string, string, string, string];

	return {
		bg: hex(bg),
		panel: hex(panel),
		ink: hex(ink),
		muted: hex(muted),
		line: rgba(ink, ALPHA.line),
		soft: rgba(ink, dark ? ALPHA.softDark : ALPHA.softLight),
		placeholder: rgba(ink, dark ? ALPHA.placeholderDark : ALPHA.placeholderLight),
		accent: hex(accent),
		onAccent: hex(onAccent),
		danger: hex(danger),
		warn: hex(warn),
		ok: hex(ok),
		selection: rgba(accent, ALPHA.selection),
		stripe: rgba(ink, ALPHA.stripe),
		lanes
	};
}

/**
 * A stable fingerprint of a desktop palette.
 *
 * `theme.id` is `family-mode`, and every followed palette has the same one —
 * `omarchy-dark` — so two different imported dark palettes are indistinguishable
 * to anything keyed off it. `LaneCanvas` invalidates its colour and portrait
 * caches from exactly that string, which would leave a graph painted in the
 * previous desktop theme's lane colours after a switch between two dark themes.
 *
 * This is what the theme store's palette **revision** is bumped from: a change
 * in the fingerprint is a change in what should be on screen, and an event that
 * re-reads an identical palette is not.
 */
export function fingerprint(palette: Palette): string {
	return [
		palette.bg,
		palette.panel,
		palette.ink,
		palette.accent,
		palette.danger,
		palette.warn,
		palette.ok,
		...palette.lanes
	].join('|');
}

/** Whether the derived palette can actually be read. For the tests and the sweep. */
export function readability(palette: Palette): { inkOnBg: number; mutedOnBg: number; labelOnAccent: number } {
	const bg = parse(palette.bg)!;
	return {
		inkOnBg: contrast(parse(palette.ink)!, bg),
		mutedOnBg: contrast(parse(palette.muted)!, bg),
		labelOnAccent: contrast(parse(palette.onAccent)!, parse(palette.accent)!)
	};
}
