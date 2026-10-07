// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { refsFitWidth, type RefsFit } from './fit';
import type { GraphRow, RefChip } from '../types';

const chip = (name: string): RefChip =>
	({ kind: 'branch', name, local: true, remotes: [], current: false, divergence: null }) as unknown as RefChip;

const row = (...names: string[]): GraphRow => ({ refs: names.map(chip) }) as unknown as GraphRow;

/** One pixel a character, so the arithmetic reads off the names. */
const fit: RefsFit = {
	measure: (text) => text.length,
	chipExtra: 20,
	maxChips: 2,
	gap: 4,
	frame: 16,
	min: 90,
	max: 400
};

describe('refsFitWidth', () => {
	it('fits the widest row, not the first', () => {
		const rows = [row('main'), row('f'.repeat(112)), row('dev')];
		// 16 frame + 112 name + 20 chip.
		expect(refsFitWidth(rows, fit)).toBe(148);
	});

	it('adds every chip a row shows, the gaps between them, and its +N', () => {
		const names = ['a'.repeat(60), 'b'.repeat(60), 'c', 'd'];
		// 16 + (60 + 20) * 2 + 4 + 4 + "+2".
		expect(refsFitWidth([row(...names)], fit)).toBe(16 + 160 + 4 + 4 + 2);
	});

	it('keeps the minimum when nothing is labelled', () => {
		expect(refsFitWidth([row(), row()], fit)).toBe(90);
	});

	it('never grows past the maximum', () => {
		expect(refsFitWidth([row('x'.repeat(1000))], fit)).toBe(400);
	});

	it('measures each name once', () => {
		let calls = 0;
		const counted = { ...fit, measure: (text: string) => (calls++, text.length) };
		refsFitWidth([row('main'), row('main'), row('main')], counted);
		expect(calls).toBe(1);
	});
});
