// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import { follow, linker, type Scrolled } from './scroll-link';

const column = (scrollTop: number, scrollHeight: number, clientHeight = 400): Scrolled => ({
	scrollTop,
	scrollHeight,
	clientHeight
});

describe('follow', () => {
	it('moves a column by the same share of what it has to scroll', () => {
		// A long side half way down: a shorter one is half way down its own.
		expect(follow(column(800, 2000), column(0, 1200))).toBe(400);
		expect(follow(column(400, 1200), column(0, 2000))).toBe(800);
	});

	it('has every column at its top together, and at its end together', () => {
		expect(follow(column(0, 2000), column(300, 1200))).toBe(0);
		expect(follow(column(1600, 2000), column(0, 1200))).toBe(800);
		// A browser may report a little past the end.
		expect(follow(column(1650, 2000), column(0, 1200))).toBe(800);
	});

	it('leaves a column with nothing to scroll at its top', () => {
		expect(follow(column(800, 2000), column(0, 400))).toBe(0);
		expect(follow(column(800, 2000), column(0, 120))).toBe(0);
	});

	it('does not move a column on account of one that cannot scroll', () => {
		expect(follow(column(0, 400), column(250, 1200))).toBe(250);
	});
});

describe('linker', () => {
	const element = (scrollHeight: number): HTMLElement =>
		({ scrollTop: 0, scrollHeight, clientHeight: 400 }) as unknown as HTMLElement;

	it('puts the other columns where the scrolled one is', () => {
		const [a, result, b] = [element(2000), element(1200), element(400)];
		a.scrollTop = 800;
		linker(() => 0).scrolled(a, [a, result, b]);
		expect([a.scrollTop, result.scrollTop, b.scrollTop]).toEqual([800, 400, 0]);
	});

	it('lets the column being scrolled lead, so a follower does not push it back', () => {
		let now = 0;
		const link = linker(() => now);
		const [a, result] = [element(2001), element(1200)];
		a.scrollTop = 801;
		link.scrolled(a, [a, result]);
		const followed = result.scrollTop;

		// The follower's own scroll event, raised by being moved, a moment later.
		now = 10;
		link.scrolled(result, [a, result]);
		expect(a.scrollTop).toBe(801);
		expect(result.scrollTop).toBe(followed);

		// Once it has settled, the other column may lead.
		now = 500;
		result.scrollTop = 0;
		link.scrolled(result, [a, result]);
		expect(a.scrollTop).toBe(0);
	});
});
