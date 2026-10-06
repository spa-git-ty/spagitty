// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import type { DiffLine, Hunk } from '../types';
import { blocksOf, blocksOfHunks, headerOf, lineRows, plainHeaderOf, rowsOf, type Row } from './rows';
import { threadsByPlace, type Thread } from './threads';

/** A file of `count` unchanged lines with one line edited at `at` (0-based). */
function edited(count: number, at: number): DiffLine[] {
	const lines: DiffLine[] = [];
	let n = 1;
	for (let i = 0; i < count; i++) {
		if (i === at) {
			lines.push({ origin: 'removed', old: n, new: null, text: `old ${n}` });
			lines.push({ origin: 'added', old: null, new: n, text: `new ${n}` });
		} else {
			lines.push({ origin: 'context', old: n, new: n, text: i === 0 ? 'fn alpha() {' : `\tline ${n}` });
		}
		n++;
	}
	return lines;
}

const kinds = (rows: Row[]) => rows.map((row) => row.kind);

describe('blocksOf', () => {
	// 30 lines: 14 unchanged, the edit (two lines), 14 unchanged.
	const lines = edited(29, 14);

	it('keeps three lines either side of a change and folds the rest', () => {
		expect(blocksOf(lines, 'changes')).toEqual([
			{ kind: 'fold', from: 0, to: 11 },
			{ kind: 'hunk', from: 11, to: 19 },
			{ kind: 'fold', from: 19, to: 30 }
		]);
	});

	it('shows an expanded fold, and the whole file as one part', () => {
		expect(blocksOf(lines, 'changes', new Set([19]))[2]).toEqual({ kind: 'plain', from: 19, to: 30 });
		// TASK-053: the changes in place, not cut into cards.
		expect(blocksOf(lines, 'whole')).toEqual([{ kind: 'hunk', from: 0, to: 30 }]);
	});

	it('shows a run too short to be worth folding', () => {
		// The edit on line 3: the two lines above it are shown, not folded.
		expect(blocksOf(edited(20, 2), 'changes')).toEqual([
			{ kind: 'hunk', from: 0, to: 7 },
			{ kind: 'fold', from: 7, to: 21 }
		]);
	});

	it('joins two changes whose gap would be too short to fold', () => {
		const two = edited(30, 8);
		two[19] = { ...two[19], origin: 'added', old: null };
		expect(blocksOf(two, 'changes').map((block) => block.kind)).toEqual(['fold', 'hunk', 'fold']);
	});

	it('folds a file whose text did not change, and has nothing to cut from none', () => {
		const same: DiffLine[] = [{ origin: 'context', old: 1, new: 1, text: 'a' }];
		expect(blocksOf(same, 'changes')).toEqual([{ kind: 'fold', from: 0, to: 1 }]);
		expect(blocksOf(same, 'whole')).toEqual([{ kind: 'plain', from: 0, to: 1 }]);
		expect(blocksOf([], 'changes')).toEqual([]);
	});

	it('names a part by its numbers and the line it sits under', () => {
		expect(headerOf(lines, { kind: 'hunk', from: 11, to: 19 })).toBe('@@ -12,7 +12,7 @@  fn alpha() {');
		expect(plainHeaderOf(lines, { kind: 'plain', from: 19, to: 30 })).toBe('unchanged · 19–29');
	});

	it('numbers a side with no lines from the line before it, as git does', () => {
		const added: DiffLine[] = [
			{ origin: 'context', old: 1, new: 1, text: 'a' },
			{ origin: 'added', old: null, new: 2, text: 'b' }
		];
		expect(headerOf(added, { kind: 'hunk', from: 1, to: 2 })).toBe('@@ -1,0 +2,1 @@  a');
	});
});

describe('blocksOfHunks', () => {
	it('lays the host’s hunks end to end, one block each', () => {
		const hunk = (start: number): Hunk => ({
			oldStart: start,
			oldLines: 1,
			newStart: start,
			newLines: 1,
			header: `@@ -${start},1 +${start},1 @@`,
			lines: [{ origin: 'added', old: null, new: start, text: 'x' }]
		});
		const laid = blocksOfHunks([hunk(3), hunk(40)]);
		expect(laid.lines).toHaveLength(2);
		expect(laid.blocks).toEqual([
			{ kind: 'hunk', from: 0, to: 1 },
			{ kind: 'hunk', from: 1, to: 2 }
		]);
	});
});

describe('rowsOf', () => {
	const lines = edited(29, 14);
	const thread: Thread = {
		id: 7,
		path: 'src/a.rs',
		line: 15,
		side: 'RIGHT',
		resolved: false,
		threadId: null,
		general: false,
		comments: []
	};

	it('gives a file a header, its folds, and each card its rows', () => {
		const rows = rowsOf([{ path: 'src/a.rs', lines, blocks: blocksOf(lines, 'changes') }]);
		expect(kinds(rows)).toEqual(['file', 'fold', 'head', ...Array(8).fill('line'), 'fold']);
		const cardLines = rows.filter((row) => row.kind === 'line');
		expect(cardLines.map((row) => row.kind === 'line' && row.last)).toEqual([
			false,
			false,
			false,
			false,
			false,
			false,
			false,
			true
		]);
		expect(lineRows(rows)).toEqual([3, 4, 5, 6, 7, 8, 9, 10]);
	});

	it('draws a thread under its line, inside that line’s card', () => {
		const rows = rowsOf(
			[{ path: 'src/a.rs', lines, blocks: blocksOf(lines, 'changes') }],
			threadsByPlace([thread])
		);
		const at = rows.findIndex((row) => row.kind === 'thread');
		const before = rows[at - 1];
		expect(before.kind === 'line' && before.line.new).toBe(15);
		expect(rows[at].kind === 'thread' && rows[at].chunk).toBe(before.kind === 'line' && before.chunk);
	});

	it('leaves a thread on a folded line to the Conversation card', () => {
		const rows = rowsOf(
			[{ path: 'src/a.rs', lines, blocks: blocksOf(lines, 'changes') }],
			threadsByPlace([{ ...thread, line: 2 }])
		);
		expect(rows.some((row) => row.kind === 'thread')).toBe(false);
	});

	it('says what it has instead of lines', () => {
		const rows = rowsOf([
			{ path: 'a.png', lines: [], blocks: [], note: 'Binary file. There are no lines to show.' },
			{ path: 'b.rs', lines: null, blocks: [] }
		]);
		expect(kinds(rows)).toEqual(['file', 'note', 'file', 'note']);
		expect(rows[3].kind === 'note' && rows[3].text).toBe('Reading…');
	});
});
