// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import {
	DEFAULT_FAMILY,
	FAMILIES,
	familyOf,
	isFamily,
	paletteOf,
	properties,
	variantOf,
	type Mode,
	type Palette
} from './themes';

/**
 * The palettes are data, so this is where they are checked — and the one thing
 * worth asserting about a colour is whether it can be read. A palette nobody
 * can see the text in is the failure mode that matters, and it is invisible in
 * a screenshot taken by someone with a good monitor in a dark room.
 *
 * Ratios are WCAG's: relative luminance, `(lighter + 0.05) / (darker + 0.05)`.
 */

type Rgba = { r: number; g: number; b: number; a: number };

function parse(colour: string): Rgba {
	const hex = colour.match(/^#([0-9a-f]{6})$/i);
	if (hex) {
		const value = parseInt(hex[1], 16);
		return { r: (value >> 16) & 255, g: (value >> 8) & 255, b: value & 255, a: 1 };
	}

	const rgba = colour.match(
		/^rgba?\(\s*([\d.]+)[\s,]+([\d.]+)[\s,]+([\d.]+)(?:[\s,/]+([\d.]+))?\s*\)$/i
	);
	if (rgba) {
		return {
			r: Number(rgba[1]),
			g: Number(rgba[2]),
			b: Number(rgba[3]),
			a: rgba[4] === undefined ? 1 : Number(rgba[4])
		};
	}

	throw new Error(`not a colour this test can read: ${colour}`);
}

/** A translucent colour is only as readable as what shows through it. */
function over(colour: string, background: string): Rgba {
	const top = parse(colour);
	const under = parse(background);
	return {
		r: top.r * top.a + under.r * (1 - top.a),
		g: top.g * top.a + under.g * (1 - top.a),
		b: top.b * top.a + under.b * (1 - top.a),
		a: 1
	};
}

function luminance({ r, g, b }: Rgba): number {
	const channel = (value: number) => {
		const scaled = value / 255;
		return scaled <= 0.03928 ? scaled / 12.92 : ((scaled + 0.055) / 1.055) ** 2.4;
	};
	return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
}

/** The colour's hue in degrees, or null for a grey. Used to tell families apart. */
function hue(colour: string): number | null {
	const { r, g, b } = parse(colour);
	const max = Math.max(r, g, b);
	const min = Math.min(r, g, b);
	if (max === min) return null;
	const d = max - min;
	const degrees =
		max === r ? ((g - b) / d) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
	return (degrees * 60 + 360) % 360;
}

function contrast(foreground: string, background: string): number {
	const front = luminance(over(foreground, background));
	const back = luminance(parse(background));
	const [lighter, darker] = front > back ? [front, back] : [back, front];
	return (lighter + 0.05) / (darker + 0.05);
}

/** Every palette in the set, with a name that says which one failed. */
const ALL: { label: string; palette: Palette }[] = FAMILIES.flatMap((family) =>
	(['light', 'dark'] as Mode[]).map((mode) => ({
		label: `${family.name} ${family[mode].name}`,
		palette: family[mode].palette
	}))
);

const TOKENS = [
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
] as const;

describe('the set', () => {
	it('is one family, Pomodoro, with a light and a dark variant (TASK-051)', () => {
		expect(FAMILIES).toHaveLength(1);
		expect(ALL).toHaveLength(2);

		for (const family of FAMILIES) {
			expect(family.light.name).not.toBe('');
			expect(family.dark.name).not.toBe('');
		}
	});

	/**
	 * The two halves of a family are the same theme in two lights, so they
	 * accent the same hue — Catppuccin's amber in both, Dracula's purple in
	 * both. The lightness differs, and has to: a light variant's accent is
	 * deepened until a white label survives on it.
	 */
	it('accents both halves of a family with the same hue', () => {
		for (const family of FAMILIES) {
			const light = hue(family.light.palette.accent);
			const dark = hue(family.dark.palette.accent);
			expect(light, `${family.name} light accent is a grey`).not.toBeNull();
			expect(dark, `${family.name} dark accent is a grey`).not.toBeNull();

			// Round the short way: 350° and 10° are twenty degrees apart.
			const apart = Math.abs(((light! - dark! + 540) % 360) - 180);
			expect(apart, `${family.name}: its two accents are different colours`).toBeLessThanOrEqual(
				30
			);
		}
	});

	it('has no two families sharing an id', () => {
		const ids = FAMILIES.map((family) => family.id);
		expect(new Set(ids).size).toBe(ids.length);
	});

	it('opens on the default family, which is first in the list', () => {
		expect(FAMILIES[0].id).toBe(DEFAULT_FAMILY);
		expect(isFamily(DEFAULT_FAMILY)).toBe(true);
	});
});

describe('every palette', () => {
	it('defines every token', () => {
		// A palette missing one would leave whatever the previous theme set,
		// which is how a half-applied theme happens.
		for (const { label, palette } of ALL) {
			for (const token of TOKENS) {
				expect(palette[token], `${label} has no ${token}`).toBeTruthy();
			}
		}
	});

	it('carries exactly five distinct lane colours', () => {
		for (const { label, palette } of ALL) {
			expect(palette.lanes, label).toHaveLength(5);
			expect(new Set(palette.lanes).size, `${label} repeats a lane colour`).toBe(5);
		}
	});

	it('uses colours this application can actually parse', () => {
		for (const { label, palette } of ALL) {
			for (const token of TOKENS) {
				expect(() => parse(palette[token]), `${label} ${token}`).not.toThrow();
			}
			for (const lane of palette.lanes) {
				expect(() => parse(lane), `${label} lane`).not.toThrow();
			}
		}
	});
});

describe('readability', () => {
	it('puts ordinary text at 4.5:1 or better against the background', () => {
		for (const { label, palette } of ALL) {
			expect(contrast(palette.ink, palette.bg), `${label} ink on bg`).toBeGreaterThanOrEqual(
				4.5
			);
			expect(
				contrast(palette.ink, palette.panel),
				`${label} ink on panel`
			).toBeGreaterThanOrEqual(4.5);
		}
	});

	it('keeps secondary text legible at 3:1, composited over what shows through', () => {
		// `muted` is translucent in every palette, so the ratio depends on what
		// is behind it — which is the whole reason this test composites.
		for (const { label, palette } of ALL) {
			expect(contrast(palette.muted, palette.bg), `${label} muted on bg`).toBeGreaterThanOrEqual(
				3
			);
		}
	});

	it('keeps text on a filled accent surface at 4.5:1', () => {
		// The primary button is the accent filled with `onAccent` on top of it.
		for (const { label, palette } of ALL) {
			expect(
				contrast(palette.onAccent, palette.accent),
				`${label} onAccent on accent`
			).toBeGreaterThanOrEqual(4.5);
		}
	});

	it('keeps the accent itself visible against the background at 3:1', () => {
		// It is a border and a link colour as well as a fill.
		for (const { label, palette } of ALL) {
			expect(
				contrast(palette.accent, palette.bg),
				`${label} accent on bg`
			).toBeGreaterThanOrEqual(3);
		}
	});

	it('keeps every semantic colour visible against the background at 3:1', () => {
		// Red, amber and green are read as *meaning* — a delete that cannot be
		// told from a save is worse than one with no colour at all — so they are
		// held to the same bar as the accent, which they sit beside.
		for (const { label, palette } of ALL) {
			for (const token of ['danger', 'warn', 'ok'] as const) {
				expect(
					contrast(palette[token], palette.bg),
					`${label} ${token} on bg`
				).toBeGreaterThanOrEqual(3);
				expect(
					contrast(palette[token], palette.panel),
					`${label} ${token} on panel`
				).toBeGreaterThanOrEqual(3);
			}
		}
	});

	it('keeps every lane colour distinguishable from the background at 3:1', () => {
		// A lane nobody can see is a graph with edges missing.
		for (const { label, palette } of ALL) {
			palette.lanes.forEach((lane, index) => {
				expect(contrast(lane, palette.bg), `${label} lane ${index + 1}`).toBeGreaterThanOrEqual(
					3
				);
			});
		}
	});
});

describe('lookup', () => {
	it('finds a family by id', () => {
		expect(familyOf('pomodoro').name).toBe('Pomodoro');
	});

	it('falls back to the default rather than failing on an id it does not know', () => {
		// The id can come from `localStorage`, which a person can edit.
		expect(familyOf('nonsense' as never).id).toBe(DEFAULT_FAMILY);
	});

	it('recognises exactly the families in the list', () => {
		for (const family of FAMILIES) expect(isFamily(family.id)).toBe(true);
		for (const nonsense of ['', 'monokai', 'catppuccin', 'nord', 'dark']) {
			expect(isFamily(nonsense), nonsense).toBe(false);
		}
	});

	it('returns the variant for the mode asked for', () => {
		expect(variantOf('pomodoro', 'light').name).toBe('Giorno');
		expect(variantOf('pomodoro', 'dark').name).toBe('Notte');
		expect(paletteOf('pomodoro', 'dark').bg).toBe('#1c1613');
	});
});

describe('properties', () => {
	it('names every token the way the stylesheet names it', () => {
		const tokens = properties(paletteOf('pomodoro', 'light'));

		expect(tokens['--bg']).toBe('#fbf7f1');
		expect(tokens['--on-accent']).toBe('#ffffff');
		expect(tokens['--lane-1']).toBe('#c23b22');
		expect(tokens['--lane-5']).toBe('#2f6fb0');
	});

	it('produces one property per token, and no others', () => {
		// Sixteen: eleven named tokens plus five lanes. A property nobody sets
		// is a colour that survives a theme change.
		for (const { label, palette } of ALL) {
			const tokens = properties(palette);

			expect(Object.keys(tokens), label).toHaveLength(TOKENS.length + 5);
			expect(Object.values(tokens).every((value) => value !== ''), label).toBe(true);
			expect(tokens['--lanes'], `${label} leaked the array`).toBeUndefined();
		}
	});
});
