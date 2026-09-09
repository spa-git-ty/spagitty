// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Turning a desktop's palette into one of Spagitty's (FEAT-080).
 *
 * The fixture is the **real** `colors.toml` from the machine this was written
 * against — `sushi-dark-palette`, as `desktop.rs` returns it. A derivation
 * tested only against an invented palette is a derivation tested against its
 * author's idea of what a palette looks like, and the whole point here is that
 * a real one turned out to carry two roles that are wrong for an application.
 *
 * The requirement is one sentence: **take the desktop's hues, derive the app's
 * roles.** Everything below is that sentence, checked.
 */

import { describe, expect, it } from 'vitest';
import { contrast, parse } from './colour';
import { fingerprint, modeOf, paletteFrom, readability, type DesktopPalette } from './omarchy';
import { paletteOf, type Palette } from './themes';

/** `sushi-dark-palette`, as the backend hands it over. */
const SUSHI: DesktopPalette = {
	mode: 'dark',
	background: '#191724',
	foreground: '#fcfcfd',
	accent: '#cf6348',
	darkBg: '#13101e',
	lighterBg: '#2a2735',
	muted: '#5f5e63',
	selectionBackground: '#cf6348',
	selectionForeground: '#191724',
	red: '#bda8a4',
	green: '#8fa487',
	yellow: '#cd9071',
	blue: '#778291',
	magenta: '#a1758a',
	cyan: '#4a89a4',
	ansi: [
		'#1d1b28',
		'#bda8a4',
		'#8fa487',
		'#cd9071',
		'#778291',
		'#a1758a',
		'#4a89a4',
		'#dfdfe0',
		'#757575',
		'#dfc9c5',
		'#afc5a7',
		'#f0b090',
		'#96a1b1',
		'#c294aa',
		'#6ba9c5',
		'#ffffff'
	]
};

/** A light desktop, because a feature tested on one mode is tested on none. */
const DAWN: DesktopPalette = {
	mode: 'light',
	background: '#faf4ed',
	foreground: '#575279',
	accent: '#d7827e',
	darkBg: '#fffaf3',
	lighterBg: '#f2e9e1',
	muted: '#9893a5',
	selectionBackground: '#dfdad9',
	selectionForeground: '#575279',
	red: '#b4637a',
	green: '#286983',
	yellow: '#ea9d34',
	blue: '#56949f',
	magenta: '#907aa9',
	cyan: '#56949f',
	ansi: Array(16).fill(null)
};

const ratio = (front: string, back: string) => contrast(parse(front)!, parse(back)!);

describe('what it takes from the desktop', () => {
	it('keeps the ground and the text exactly as the desktop has them', () => {
		const palette = paletteFrom(SUSHI);

		expect(palette.bg).toBe('#191724');
		expect(palette.ink).toBe('#fcfcfd');
	});

	it('takes chrome from the desktop where it names one', () => {
		expect(paletteFrom(SUSHI).panel).toBe('#13101e');
	});

	it('takes the accent, and the graph lanes, from the desktop hues', () => {
		const palette = paletteFrom(SUSHI);

		expect(palette.accent).toBe('#cf6348');
		// blue, magenta, red, cyan, green — the order the built-in families use.
		expect(palette.lanes).toEqual([
			'#778291',
			'#a1758a',
			'#bda8a4',
			'#4a89a4',
			'#8fa487'
		]);
	});

	it('reads the desktop mode where the file says so', () => {
		expect(modeOf(SUSHI)).toBe('dark');
		expect(modeOf(DAWN)).toBe('light');
	});

	/** No `mode` key: the ground's own luminance is the only honest evidence. */
	it('infers the mode from the background where the file does not say', () => {
		expect(modeOf({ ...SUSHI, mode: null })).toBe('dark');
		expect(modeOf({ ...DAWN, mode: null })).toBe('light');
	});
});

describe('what it refuses to take literally', () => {
	/**
	 * **The measurement this whole file exists for.** The desktop's `muted` is
	 * 2.75:1 on its background — a terminal comment colour — and it is what
	 * every timestamp, path and secondary label in Spagitty would have been
	 * painted with.
	 */
	it('lifts the desktop muted onto the readable line', () => {
		expect(ratio('#5f5e63', '#191724')).toBeLessThan(4.5);

		const palette = paletteFrom(SUSHI);

		expect(palette.muted).not.toBe('#5f5e63');
		expect(ratio(palette.muted, palette.bg)).toBeGreaterThanOrEqual(4.5);
	});

	/**
	 * The other one. Light text on this accent is 3.70:1 and the dark ground is
	 * 4.65:1, so a filled button's label is the *background* colour — which is
	 * not what anybody writing this by hand would have chosen.
	 */
	it('measures the label on a filled accent rather than assuming white', () => {
		const palette = paletteFrom(SUSHI);

		expect(palette.onAccent).toBe('#191724');
		expect(ratio(palette.onAccent, palette.accent)).toBeGreaterThanOrEqual(4.5);
	});

	it('does the same on a light desktop, where the answer is the other one', () => {
		const palette = paletteFrom(DAWN);

		expect(ratio(palette.onAccent, palette.accent)).toBeGreaterThanOrEqual(4.5);
		expect(ratio(palette.muted, palette.bg)).toBeGreaterThanOrEqual(4.5);
	});
});

/**
 * The same bars `themes.test.ts` holds the eight built-in families to. A
 * followed palette is a palette, and it does not get a lower standard because
 * it arrived at runtime.
 */
describe('a followed palette meets the contrast the built-in ones do', () => {
	const cases: [string, DesktopPalette][] = [
		['sushi-dark-palette', SUSHI],
		['a light desktop', DAWN]
	];

	it.each(cases)('%s: ordinary text clears 4.5:1', (_label, desktop) => {
		const palette = paletteFrom(desktop);

		expect(ratio(palette.ink, palette.bg)).toBeGreaterThanOrEqual(4.5);
		expect(ratio(palette.ink, palette.panel)).toBeGreaterThanOrEqual(4.5);
		expect(ratio(palette.muted, palette.bg)).toBeGreaterThanOrEqual(4.5);
	});

	it.each(cases)('%s: indicators clear 3:1', (_label, desktop) => {
		const palette = paletteFrom(desktop);

		for (const token of ['accent', 'danger', 'warn', 'ok'] as const) {
			expect(ratio(palette[token], palette.bg), token).toBeGreaterThanOrEqual(3);
		}
		palette.lanes.forEach((lane, index) => {
			expect(ratio(lane, palette.bg), `lane ${index + 1}`).toBeGreaterThanOrEqual(3);
		});
	});

	/** A lane you cannot tell from the one beside it is a branch you cannot
	 *  follow, and a desktop palette is likelier to be low-contrast internally
	 *  than a hand-picked one. */
	it.each(cases)('%s: the five lanes are distinguishable from each other', (_label, desktop) => {
		const { lanes } = paletteFrom(desktop);
		expect(new Set(lanes).size).toBe(5);
	});
});

describe('what it does with a palette that is missing things', () => {
	const bare: DesktopPalette = {
		mode: null,
		background: '#101010',
		foreground: '#f0f0f0',
		accent: null,
		darkBg: null,
		lighterBg: null,
		muted: null,
		selectionBackground: null,
		selectionForeground: null,
		red: null,
		green: null,
		yellow: null,
		blue: null,
		magenta: null,
		cyan: null,
		ansi: Array(16).fill(null)
	};

	/**
	 * Total. A palette with a field left out falls through to whatever the
	 * previous theme set, which is how half a theme ends up on screen — the
	 * reason `themes.ts` makes every field required.
	 */
	it('still produces every token', () => {
		const palette = paletteFrom(bare);
		const keys: (keyof Palette)[] = [
			'bg',
			'panel',
			'ink',
			'muted',
			'line',
			'soft',
			'placeholder',
			'accent',
			'onAccent',
			'danger',
			'warn',
			'ok',
			'selection',
			'stripe'
		];

		for (const key of keys) expect(palette[key], key).toBeTruthy();
		expect(palette.lanes).toHaveLength(5);
	});

	it('derives chrome and secondary text from the ground and the ink', () => {
		const palette = paletteFrom(bare);

		expect(palette.panel).not.toBe(palette.bg);
		expect(ratio(palette.muted, palette.bg)).toBeGreaterThanOrEqual(4.5);
	});

	/** Five colours, not three: two branches in one colour is worse than two
	 *  colours borrowed from the default family. */
	it('borrows the default family for lanes the desktop names none of', () => {
		const palette = paletteFrom(bare);
		expect(new Set(palette.lanes).size).toBe(5);
	});

	/**
	 * An ANSI palette is positional. A generator that names `cyan` but not
	 * `blue` must not have its cyan slide into the blue lane.
	 */
	it('uses an ANSI slot only for the hue that slot means', () => {
		const partial: DesktopPalette = {
			...bare,
			ansi: [null, null, null, null, '#4444ff', null, null, null, ...Array(8).fill(null)]
		};

		expect(paletteFrom(partial).lanes[0]).toBe('#4444ff');
	});
});

/**
 * The reason a revision exists at all.
 *
 * `theme.id` is `family-mode`, so every followed palette is `omarchy-dark` and
 * a switch between two dark desktop themes is invisible to anything keyed off
 * it — including `LaneCanvas`, which invalidates its lane colours and its whole
 * portrait cache from exactly that string.
 */
describe('telling two palettes apart', () => {
	it('differs between two desktop palettes that share a mode', () => {
		const other: DesktopPalette = { ...SUSHI, background: '#101018', accent: '#6ba9c5' };

		expect(fingerprint(paletteFrom(SUSHI))).not.toBe(fingerprint(paletteFrom(other)));
	});

	it('is stable for the same palette read twice', () => {
		expect(fingerprint(paletteFrom(SUSHI))).toBe(fingerprint(paletteFrom({ ...SUSHI })));
	});

	/** A lane change alone is a repaint, and the graph is where it shows. */
	it('moves when only a lane colour changes', () => {
		const recoloured: DesktopPalette = { ...SUSHI, blue: '#123456' };

		expect(fingerprint(paletteFrom(SUSHI))).not.toBe(fingerprint(paletteFrom(recoloured)));
	});

	it('tells a built-in family from a desktop palette', () => {
		expect(fingerprint(paletteOf('catppuccin', 'dark'))).not.toBe(
			fingerprint(paletteFrom(SUSHI))
		);
	});
});

/** The numbers the sweep asks for, so a person can read them off a screen. */
describe('reporting its own readability', () => {
	it('reports the three ratios that decided the design', () => {
		const measured = readability(paletteFrom(SUSHI));

		expect(measured.inkOnBg).toBeCloseTo(17.22, 1);
		expect(measured.mutedOnBg).toBeGreaterThanOrEqual(4.5);
		expect(measured.labelOnAccent).toBeGreaterThanOrEqual(4.5);
	});
});
