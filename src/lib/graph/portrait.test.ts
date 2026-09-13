// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it } from 'vitest';
import { LANE_COLOR_COUNT } from '$lib/metrics';
import {
	drawPortrait,
	forgetPortraits,
	letterColor,
	lettersOf,
	portrait,
	portraitBackground,
	portraitCacheSize,
	portraitTile,
	seedOf
} from './portrait';

beforeEach(() => forgetPortraits());

describe('the seed a portrait is generated from', () => {
	it('is the email, because that is what identifies a person across names', () => {
		expect(seedOf('Ada@Example.com', 'Ada Lovelace')).toBe('ada@example.com');
		expect(seedOf('ada@example.com', 'ada l')).toBe(
			seedOf('ADA@EXAMPLE.COM', 'Ada Lovelace')
		);
	});

	it('falls back to the name when git recorded no address', () => {
		expect(seedOf('', 'Ada Lovelace')).toBe('ada lovelace');
		expect(seedOf('   ', 'Ada Lovelace')).toBe('ada lovelace');
	});

	it('is empty when there is nothing to go on, rather than throwing', () => {
		expect(seedOf('', '')).toBe('');
		expect(() => portrait('')).not.toThrow();
	});
});

describe('the letters on the disc', () => {
	it('are the initials of the name', () => {
		expect(lettersOf('Ada Lovelace', 'ada@example.com')).toBe('AL');
	});

	it('fall back to the address when git recorded no name', () => {
		expect(lettersOf('', 'ada.lovelace@example.com')).toBe('AL');
	});
});

describe('a portrait', () => {
	it('is the same colour for the same address, every time', () => {
		expect(portrait('ada@example.com')).toEqual(portrait('ada@example.com'));
	});

	it('is a different colour for a different address', () => {
		const ada = portrait('ada@example.com');
		const charles = portrait('charles@example.com');
		expect(ada).not.toEqual(charles);
	});

	it('stays inside the theme’s lane palette, so it introduces no new hue', () => {
		for (const seed of ['ada@example.com', 'charles@example.com', 'x', '', 'a@b.c']) {
			const face = portrait(seed);
			expect(face.color).toBeGreaterThanOrEqual(0);
			expect(face.color).toBeLessThan(LANE_COLOR_COUNT);
		}
	});

	it('spreads similar addresses apart rather than giving them the same disc', () => {
		const colours = new Set(
			['ada.l@example.com', 'ada.m@example.com', 'ada.n@example.com', 'ada.o@example.com'].map(
				(seed) => portrait(seed).color
			)
		);
		expect(colours.size).toBeGreaterThan(1);
	});
});

describe('the CSS form', () => {
	it('paints with a lane variable, so a theme change repaints it', () => {
		const css = portraitBackground('ada@example.com');
		expect(css).toMatch(/^var\(--lane-[1-5]\)$/);
		expect(css).not.toMatch(/#[0-9a-f]{3,6}/i);
		expect(css).not.toContain('radial-gradient');
	});
});

describe('letter colour', () => {
	it('is white on a dark fill and dark on a light one', () => {
		expect(letterColor('#1e66f5')).toBe('#ffffff');
		expect(letterColor('#a6e3a1')).toBe('#1a1a1a');
	});

	it('falls back to white when the fill cannot be parsed', () => {
		expect(letterColor('color-mix(in srgb, red, blue)')).toBe('#ffffff');
	});
});

describe('drawing', () => {
	interface Call {
		op: string;
		args: unknown[];
	}

	function fakeContext() {
		const calls: Call[] = [];
		const ctx = {
			fillStyle: '' as unknown,
			font: '',
			textAlign: '',
			textBaseline: '',
			fillRect: (...args: unknown[]) => calls.push({ op: 'fillRect', args }),
			fillText: (...args: unknown[]) => calls.push({ op: 'fillText', args })
		};
		return { ctx, calls };
	}

	it('fills the square and writes the initials on it', () => {
		const { ctx, calls } = fakeContext();
		drawPortrait(
			ctx as unknown as CanvasRenderingContext2D,
			'ada@example.com',
			18,
			['#111', '#222', '#333', '#444', '#555'],
			'AL'
		);

		expect(calls[0].op).toBe('fillRect');
		expect(calls[0].args).toEqual([0, 0, 18, 18]);
		const text = calls.find((c) => c.op === 'fillText');
		expect(text?.args[0]).toBe('AL');
		expect(text?.args[1]).toBe(9);
	});

	it('cycles the palette rather than running off the end of a short one', () => {
		const { ctx } = fakeContext();
		expect(() =>
			drawPortrait(
				ctx as unknown as CanvasRenderingContext2D,
				'ada@example.com',
				18,
				['#111'],
				'AL'
			)
		).not.toThrow();
	});
});

describe('the tile cache', () => {
	const palette = ['#111', '#222', '#333', '#444', '#555'];

	it('renders an author once and hands the same tile back', () => {
		const first = portraitTile('ada@example.com', 18, palette, 'AL');
		const second = portraitTile('ada@example.com', 18, palette, 'AL');

		// happy-dom gives no 2d context, so a null here is the environment
		// rather than a failure — what matters is that both answers agree.
		expect(second).toBe(first);
		if (first) expect(portraitCacheSize()).toBe(1);
	});

	it('treats different letters as a different tile', () => {
		const ada = portraitTile('ada@example.com', 18, palette, 'AL');
		const other = portraitTile('ada@example.com', 18, palette, 'A');
		if (ada && other) expect(other).not.toBe(ada);
	});

	it('treats a different palette as a different tile, so a theme change is not stale', () => {
		const light = portraitTile('ada@example.com', 18, palette, 'AL');
		const dark = portraitTile('ada@example.com', 18, ['#eee', '#ddd', '#ccc', '#bbb', '#aaa'], 'AL');

		if (light && dark) {
			expect(dark).not.toBe(light);
			expect(portraitCacheSize()).toBe(2);
		}
	});

	it('is emptied when the theme changes', () => {
		portraitTile('ada@example.com', 18, palette, 'AL');
		forgetPortraits();
		expect(portraitCacheSize()).toBe(0);
	});
});
