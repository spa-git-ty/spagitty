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
		linker().scrolled(a, [a, result, b]);
		expect([a.scrollTop, result.scrollTop, b.scrollTop]).toEqual([800, 400, 0]);
	});

	it('takes a follower’s own scroll event for what it is, and does not push the leader back', () => {
		const link = linker();
		const [a, result] = [element(2001), element(1200)];
		a.scrollTop = 801;
		link.scrolled(a, [a, result]);
		const followed = result.scrollTop;

		// The event raised on the follower by being moved. A browser may have
		// kept a part of a pixel.
		result.scrollTop = followed - 0.4;
		link.scrolled(result, [a, result]);
		expect(a.scrollTop).toBe(801);
	});

	it('lets the person lead from another column straight away', () => {
		const link = linker();
		const [a, result] = [element(2000), element(1200)];
		a.scrollTop = 800;
		link.scrolled(a, [a, result]);
		expect(result.scrollTop).toBe(400);

		// The person turns to the result before its echo has even arrived.
		result.scrollTop = 800;
		link.scrolled(result, [a, result]);
		expect(a.scrollTop).toBe(1600);

		// And the leader's echo of that, in turn, moves nothing.
		link.scrolled(a, [a, result]);
		expect(result.scrollTop).toBe(800);
	});

	it('does not wait for an event from a column it did not have to move', () => {
		const link = linker();
		const [a, result] = [element(2000), element(1200)];
		// Both at their tops already: nothing is moved, and nothing is expected.
		link.scrolled(a, [a, result]);
		result.scrollTop = 400;
		link.scrolled(result, [a, result]);
		expect(a.scrollTop).toBe(800);
	});
});
