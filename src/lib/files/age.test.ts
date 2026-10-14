// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { BAR_MAX, BAR_MIN, shortAge, strengths } from './age';

const NOW = 1_800_000_000_000;
const ago = (seconds: number) => NOW / 1000 - seconds;

describe('shortAge', () => {
	it('says an age in a number and a unit', () => {
		expect(shortAge(ago(10), NOW)).toBe('now');
		expect(shortAge(ago(5 * 60), NOW)).toBe('5 min');
		expect(shortAge(ago(3 * 3600), NOW)).toBe('3 h');
		expect(shortAge(ago(2 * 86400), NOW)).toBe('2 d');
		expect(shortAge(ago(92 * 86400), NOW)).toBe('3 mo');
		expect(shortAge(ago(210 * 86400), NOW)).toBe('7 mo');
		expect(shortAge(ago(800 * 86400), NOW)).toBe('2 y');
	});

	it('never says nothing of a unit, or an age in the future', () => {
		expect(shortAge(ago(50), NOW)).toBe('1 min');
		expect(shortAge(ago(-500), NOW)).toBe('now');
		expect(shortAge(ago(365 * 86400), NOW)).toBe('1 y');
	});
});

describe('strengths', () => {
	it('draws the oldest faintest and the newest strongest', () => {
		const [old, middle, recent] = strengths([100, 150, 200]);

		expect(old).toBe(BAR_MIN);
		expect(recent).toBe(BAR_MAX);
		expect(middle).toBeCloseTo((BAR_MIN + BAR_MAX) / 2);
	});

	/** Every row has an age bar: none is ever drawn at nothing. */
	it('never draws a bar at nothing', () => {
		for (const strength of strengths([5, 9, 1, 300, 12])) {
			expect(strength).toBeGreaterThanOrEqual(BAR_MIN);
			expect(strength).toBeLessThanOrEqual(BAR_MAX);
		}
	});

	it('draws a file one commit wrote at full strength', () => {
		expect(strengths([42])).toEqual([BAR_MAX]);
		expect(strengths([7, 7, 7])).toEqual([BAR_MAX, BAR_MAX, BAR_MAX]);
		expect(strengths([])).toEqual([]);
	});
});
