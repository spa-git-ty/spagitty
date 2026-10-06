// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import {
	allFrom,
	badgesOf,
	chosenLines,
	counts,
	editSeed,
	fingerprint,
	kept,
	landing,
	layout,
	locate,
	offered,
	parseRegions,
	pickAll,
	placeholder,
	readFile,
	resultText,
	splitLines,
	statusLabel,
	toggled,
	why,
	type Choice
} from './model';

const names = { a: 'main', b: 'feat' };

/** Two regions; feat added a line above the second, so the sides number it differently. */
const MERGED = [
	'1',
	'<<<<<<< main',
	'TWO main',
	'||||||| base',
	'two',
	'=======',
	'TWO feat',
	'TWO feat, again',
	'>>>>>>> feat',
	'3',
	'4',
	'<<<<<<< main',
	'FIVE main',
	'||||||| base',
	'five',
	'=======',
	'FIVE feat',
	'>>>>>>> feat',
	'6',
	''
].join('\n');
const A = '1\nTWO main\n3\n4\nFIVE main\n6\n';
const B = '1\nTWO feat\nTWO feat, again\n3\nadded\n4\nFIVE feat\n6\n';

const file = readFile({ path: 'src/f.ts', kind: 'bothModified', merged: MERGED, a: A, b: B, aExists: true, bExists: true });

describe('reading a file', () => {
	it('splits on newlines alone and remembers the last one', () => {
		expect(splitLines('a\r\nb\r\n')).toEqual({ lines: ['a\r', 'b\r'], eol: true });
		expect(splitLines('a')).toEqual({ lines: ['a'], eol: false });
		expect(splitLines('')).toEqual({ lines: [], eol: false });
	});

	it('reads each region with its base, as strictly as the backend', () => {
		expect(file.regions.map((r) => [r.a, r.base, r.b])).toEqual([
			[['TWO main'], ['two'], ['TWO feat', 'TWO feat, again']],
			[['FIVE main'], ['five'], ['FIVE feat']]
		]);
		expect(parseRegions(['<<<<<<< a', 'x', '======='])).toEqual([]);
	});

	it('finds each region in each side’s own file', () => {
		expect(file.regions.map((r) => [r.aLine, r.bLine])).toEqual([
			[2, 2],
			[5, 7]
		]);
		expect(locate(['a', 'x', 'b', 'x'], ['b'], ['x'], 0)).toEqual({ line: 4, next: 4 });
	});

	it('takes what the backend found over what it would locate', () => {
		const sourced = readFile({
			path: 'f',
			kind: 'bothModified',
			merged: MERGED,
			a: A,
			b: B,
			aExists: true,
			bExists: true,
			sources: [{ index: 0, aLine: 9, bLine: 8, aFrom: 'e41c0b7 · fix', bFrom: null }]
		});
		expect(sourced.regions[0]).toMatchObject({ aLine: 9, bLine: 8, aFrom: 'e41c0b7 · fix' });
	});

	it('a file deleted on one side is one region, chosen whole', () => {
		const gone = readFile({ path: 'g', kind: 'deletedByThem', merged: null, a: 'x\n', b: null, aExists: true, bExists: false });
		expect(gone.whole).toBe(true);
		expect(gone.regions).toHaveLength(1);
		expect(offered(gone)).toEqual(['a', 'b']);
		expect(why(gone.regions[0], gone, names)).toBe('feat deleted this file; main changed it.');
		expect(landing(gone, [{ mode: 'b' }])).toEqual({ path: 'g', take: 'theirs' });
	});
});

describe('every way out of a region', () => {
	const [first] = file.regions;
	const lines = (choice: Choice | null) => chosenLines(first, choice)?.map((l) => `${l.from}:${l.text}`);

	it('takes a side, both in either order, picked lines, or typed text', () => {
		expect(lines({ mode: 'a' })).toEqual(['a:TWO main']);
		expect(lines({ mode: 'b' })).toEqual(['b:TWO feat', 'b:TWO feat, again']);
		expect(lines({ mode: 'ab' })).toEqual(['a:TWO main', 'b:TWO feat', 'b:TWO feat, again']);
		expect(lines({ mode: 'ba' })).toEqual(['b:TWO feat', 'b:TWO feat, again', 'a:TWO main']);
		expect(lines({ mode: 'pick', a: [true], b: [false, true] })).toEqual(['a:TWO main', 'b:TWO feat, again']);
		expect(lines({ mode: 'edit', text: 'mine\nmine too' })).toEqual(['mine:mine', 'mine:mine too']);
		expect(lines({ mode: 'edit', text: '' })).toEqual([]);
		expect(lines(null)).toBeUndefined();
	});

	it('says which lines will not land', () => {
		expect(kept({ mode: 'a' }, 'b', 0)).toBe(false);
		expect(kept({ mode: 'ba' }, 'a', 0)).toBe(true);
		expect(kept({ mode: 'pick', a: [false], b: [true] }, 'a', 0)).toBe(false);
		expect(kept(null, 'a', 0)).toBeNull();
	});

	it('starts Pick lines with every line ticked, and turns one over', () => {
		const all = pickAll(first) as Choice & { mode: 'pick' };
		expect(all).toEqual({ mode: 'pick', a: [true], b: [true, true] });
		expect(toggled(all, 'b', 0)).toEqual({ mode: 'pick', a: [true], b: [false, true] });
		expect(all.b[0]).toBe(true);
	});

	it('starts a hand edit from the result so far, or both sides', () => {
		expect(editSeed(first, { mode: 'b' })).toEqual({ mode: 'edit', text: 'TWO feat\nTWO feat, again' });
		expect(editSeed(first, null)).toEqual({ mode: 'edit', text: 'TWO main\nTWO feat\nTWO feat, again' });
	});

	it('names each choice and its badges', () => {
		expect(statusLabel({ mode: 'ab' }, names)).toBe('Both, main first');
		expect(statusLabel({ mode: 'ba' }, names)).toBe('Both, feat first');
		expect(statusLabel(null, names)).toBe('Unresolved');
		expect(badgesOf({ mode: 'ba' })).toEqual(['b', 'a']);
		expect(placeholder(first, names)).toBe('1 line from main, 2 lines from feat. Pick below, or type your own.');
	});
});

describe('the result', () => {
	it('is the file once every region is chosen, line endings and all', () => {
		expect(resultText(file, [{ mode: 'a' }, null])).toBeNull();
		expect(resultText(file, [{ mode: 'a' }, { mode: 'b' }])).toBe('1\nTWO main\n3\n4\nFIVE feat\n6\n');
		const crlf = readFile({ path: 'w', kind: 'bothModified', merged: MERGED.replace(/\n/g, '\r\n'), a: null, b: null, aExists: true, bExists: true });
		expect(resultText(crlf, [{ mode: 'a' }, { mode: 'a' }])).toBe('1\r\nTWO main\r\n3\r\n4\r\nFIVE main\r\n6\r\n');
	});

	it('numbers later lines by the choices made above them', () => {
		const before = layout(file, [null, null]);
		expect(before[0].resultStart).toBe(2);
		expect(before[1].resultStart).toBe(4);
		expect(before[1].before).toEqual([
			{ n: 2, text: '3' },
			{ n: 3, text: '4' }
		]);
		const after = layout(file, [{ mode: 'ab' }, null]);
		expect(after[1].resultStart).toBe(7);
		expect(after[1].after).toEqual([{ n: 7, text: '6' }]);
	});

	it('shows each side’s context from its own file, by its own numbers', () => {
		const [, second] = layout(file, [null, null]);
		expect(second.bBefore).toEqual([
			{ n: 4, text: '3' },
			{ n: 5, text: 'added' },
			{ n: 6, text: '4' }
		]);
		expect(second.aAfter).toEqual([{ n: 6, text: '6' }]);
	});

	it('counts what is resolved, and every region of a file can come from one side', () => {
		expect(counts([file], { 'src/f.ts': [{ mode: 'a' }, null] })).toEqual({ total: 2, resolved: 1 });
		expect(allFrom(file, 'b')).toEqual([{ mode: 'b' }, { mode: 'b' }]);
		expect(landing(file, allFrom(file, 'a'))).toEqual({ path: 'src/f.ts', text: A });
	});

	it('fingerprints a region by its sides', () => {
		expect(fingerprint(file.regions[0])).toMatch(/^[0-9a-f]{8}$/);
		expect(fingerprint(file.regions[0])).not.toBe(fingerprint(file.regions[1]));
	});

	it('says why each conflicts', () => {
		expect(why(file.regions[1], file, names)).toBe('Both rewrote the same line.');
		expect(why(file.regions[0], file, names)).toBe('Both changed these lines since they split.');
	});
});
