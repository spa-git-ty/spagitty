// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The colour arithmetic, pinned to published numbers rather than to itself.
 *
 * `themes.test.ts` has its own `parse`, `luminance` and `contrast`, and this
 * file deliberately does not share them — a test that checked the palettes
 * using the application's own contrast function would agree with itself and
 * pass whatever the function did. So both implementations exist, and this one
 * is held to WCAG's worked examples and to values computed by hand.
 */

import { describe, expect, it } from 'vitest';
import {
	contrast,
	hex,
	isDark,
	luminance,
	mix,
	mostReadable,
	parse,
	rgba,
	untilReadable
} from './colour';

const BLACK = { r: 0, g: 0, b: 0 };
const WHITE = { r: 255, g: 255, b: 255 };

describe('parsing', () => {
	it('reads the three shapes a palette actually uses', () => {
		expect(parse('#191724')).toEqual({ r: 25, g: 23, b: 36 });
		expect(parse('191724')).toEqual({ r: 25, g: 23, b: 36 });
		expect(parse('rgba(76, 79, 105, 0.72)')).toEqual({ r: 76, g: 79, b: 105 });
	});

	/** `#abc` is `#aabbcc`. Reading it as `#0a0b0c` would darken every short
	 *  hex a generator wrote, which is a bug that looks like a design choice. */
	it('expands a three-digit hex the way CSS does', () => {
		expect(parse('#abc')).toEqual({ r: 170, g: 187, b: 204 });
	});

	it('is case-insensitive and tolerates surrounding space', () => {
		expect(parse('  #CF6348 ')).toEqual(parse('#cf6348'));
	});

	/**
	 * Null rather than a throw. Every caller is deriving a palette from data
	 * that may be malformed, and the answer to one bad value is another
	 * candidate, not a lost theme.
	 */
	it.each(['', 'red', '#19172', '#gggggg', 'url(x)', 'color-mix(in srgb, red, blue)'])(
		'refuses %s',
		(value) => {
			expect(parse(value)).toBeNull();
		}
	);
});

describe('formatting', () => {
	it('rounds and clamps into six hex digits', () => {
		expect(hex({ r: 25.4, g: 23.5, b: 36 })).toBe('#191824');
		expect(hex({ r: -10, g: 300, b: 0 })).toBe('#00ff00');
	});

	it('writes a wash the way the palettes already spell one', () => {
		expect(rgba({ r: 76, g: 79, b: 105 }, 0.72)).toBe('rgba(76, 79, 105, 0.72)');
	});
});

describe('contrast', () => {
	/** WCAG's own extremes. Black on white is 21:1 and anything on itself is 1. */
	it('agrees with the published bounds', () => {
		expect(contrast(BLACK, WHITE)).toBeCloseTo(21, 5);
		expect(contrast(WHITE, WHITE)).toBeCloseTo(1, 5);
	});

	it('is symmetrical', () => {
		const a = { r: 207, g: 99, b: 72 };
		expect(contrast(a, BLACK)).toBeCloseTo(contrast(BLACK, a), 10);
	});

	/**
	 * The four measurements the Omarchy work is built on, recomputed here.
	 *
	 * They come from `sushi-dark-palette` and they are the whole argument for
	 * deriving rather than copying: two of the four fail.
	 */
	it.each([
		['foreground on background', '#fcfcfd', '#191724', 17.22],
		['muted on background', '#5f5e63', '#191724', 2.75],
		['foreground on accent', '#fcfcfd', '#cf6348', 3.7],
		['background on accent', '#191724', '#cf6348', 4.65]
	])('%s is %s:1', (_label, front, back, expected) => {
		expect(contrast(parse(front)!, parse(back)!)).toBeCloseTo(expected, 1);
	});

	/**
	 * Mid grey has a luminance of 0.216, so a threshold taken on the channel
	 * average calls it dark and puts white text on it at 3.9:1.
	 */
	it('calls mid grey light', () => {
		expect(isDark(parse('#808080')!)).toBe(false);
		expect(isDark(parse('#191724')!)).toBe(true);
		expect(isDark(parse('#faf4ed')!)).toBe(false);
	});

	it('computes luminance from the sRGB curve, not the channel average', () => {
		// Green carries most of the weight; equal channel values do not give
		// equal luminance.
		expect(luminance({ r: 0, g: 255, b: 0 })).toBeCloseTo(0.7152, 4);
		expect(luminance({ r: 0, g: 0, b: 255 })).toBeCloseTo(0.0722, 4);
	});
});

describe('mixing', () => {
	it('is linear between the two ends', () => {
		expect(mix(WHITE, BLACK, 0)).toEqual(BLACK);
		expect(mix(WHITE, BLACK, 1)).toEqual(WHITE);
		expect(hex(mix(WHITE, BLACK, 0.5))).toBe('#808080');
	});

	it('clamps rather than extrapolating', () => {
		expect(mix(WHITE, BLACK, 5)).toEqual(WHITE);
		expect(mix(WHITE, BLACK, -5)).toEqual(BLACK);
	});
});

describe('choosing a readable colour', () => {
	/**
	 * The decision that white would have got wrong. On this accent the light
	 * foreground is 3.70:1 and the dark background is 4.65:1, so the label on a
	 * filled button is the *background* colour — which nobody would write down.
	 */
	it('picks the label a filled accent can actually carry', () => {
		const accent = parse('#cf6348')!;
		const chosen = mostReadable(accent, [parse('#fcfcfd')!, parse('#191724')!]);

		expect(hex(chosen)).toBe('#191724');
	});

	it('leaves a tie with the first candidate, so a palette keeps its preference', () => {
		const chosen = mostReadable(parse('#808080')!, [parse('#000000')!, parse('#000000')!]);
		expect(hex(chosen)).toBe('#000000');
	});
});

describe('walking a colour until it can be read', () => {
	/** The one that makes a desktop palette usable rather than merely faithful. */
	it('lifts an unreadable muted onto the 4.5:1 line', () => {
		const bg = parse('#191724')!;
		const ink = parse('#fcfcfd')!;
		const muted = parse('#5f5e63')!;

		expect(contrast(muted, bg)).toBeLessThan(4.5);

		const lifted = untilReadable(muted, ink, bg, 4.5);

		expect(contrast(lifted, bg)).toBeGreaterThanOrEqual(4.5);
		// And not simply the foreground: it stops at the first step that
		// clears, so the desktop's own hue survives as far as it can.
		expect(hex(lifted)).not.toBe(hex(ink));
	});

	it('returns a colour that already clears, untouched', () => {
		const bg = parse('#191724')!;
		const ink = parse('#fcfcfd')!;

		expect(untilReadable(ink, ink, bg, 4.5)).toBe(ink);
	});

	/**
	 * There is no null return, and there must not be: a caller mid-repaint has
	 * to have a colour, and the closest legible thing beats the original.
	 */
	it('gives the target when nothing on the way there clears', () => {
		const bg = WHITE;
		const impossible = untilReadable(WHITE, WHITE, bg, 21);
		expect(impossible).toEqual(WHITE);
	});
});
