// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { nameAt } from './usages';

describe('the name a click lands on (FEAT-154)', () => {
	it('is the whole name around the column, from its first letter to just past its last', () => {
		const line = '    let total = sum_of(items.$count);';
		expect(nameAt(line, 8)).toBe('total');
		expect(nameAt(line, 13)).toBe('total');
		expect(nameAt(line, 16)).toBe('sum_of');
		expect(nameAt(line, 30)).toBe('$count');
	});

	it('is nothing on a space, a symbol or a number', () => {
		expect(nameAt('    a = 42;', 1)).toBeNull();
		expect(nameAt('a = 42;', 2)).toBeNull();
		expect(nameAt('a = 42;', 5)).toBeNull();
		expect(nameAt('', 0)).toBeNull();
	});

	it('reads names in any script', () => {
		expect(nameAt('const größe = 1', 8)).toBe('größe');
	});
});
