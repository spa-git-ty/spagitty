// SPDX-License-Identifier: GPL-3.0-or-later
import { expect, it } from 'vitest';
import type { ConflictFix, DiffLine } from '../types';
import { blocksOf, rowsOf, type Row } from './rows';

/** 40 lines, line 10 and line 30 each replaced: two changed parts. */
function twoEdits(): DiffLine[] {
	const lines: DiffLine[] = [];
	for (let n = 1; n <= 40; n++) {
		if (n === 10 || n === 30) {
			lines.push({ origin: 'removed', old: n, new: null, text: `old ${n}` });
			lines.push({ origin: 'added', old: null, new: n, text: `new ${n}` });
		} else {
			lines.push({ origin: 'context', old: n, new: n, text: `line ${n}` });
		}
	}
	return lines;
}

const FIX: ConflictFix = {
	path: 'a.rs',
	merge: '7c1e9a0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
	short: '7c1e9a0',
	summary: "Merge branch 'main' into feat",
	lines: [30],
	mainSide: ['main thirty'],
	branchSide: ['branch thirty']
};

function laid(sides = new Map<string, 'main' | 'branch'>()): Row[] {
	const lines = twoEdits();
	return rowsOf([{ path: 'a.rs', lines, blocks: blocksOf(lines, 'changes'), fixes: [FIX] }], new Map(), sides);
}

it('frames the changed part a merge wrote as a conflict fix, and only that one', () => {
	const heads = laid().filter((row) => row.kind === 'head');
	expect(heads.map((row) => row.kind === 'head' && row.tone)).toEqual(['hunk', 'conflict']);
	expect(heads[1].kind === 'head' && heads[1].fix).toBe(FIX);

	const fixed = laid().filter((row) => row.kind === 'line' && row.fixed);
	expect(fixed.map((row) => row.kind === 'line' && row.line.text)).toEqual(['new 30']);
	// The rest of the card is the card's, not the fix's.
	const card = laid().filter((row) => row.kind === 'line' && row.tone === 'conflict');
	expect(card.length).toBeGreaterThan(1);
});

it('opens a side of the conflict under the card’s header', () => {
	const head = laid().find((row) => row.kind === 'head' && row.fix);
	const chunk = head!.kind === 'head' ? head!.chunk : '';
	const rows = laid(new Map([[chunk, 'main']]));
	const at = rows.findIndex((row) => row.kind === 'side');
	expect(rows[at - 1]).toMatchObject({ kind: 'head', chunk });
	expect(rows[at]).toMatchObject({ kind: 'side', side: 'main', fix: FIX, tone: 'conflict' });
});

it('leaves a part alone that a fix wrote no added line of', () => {
	const lines = twoEdits();
	const elsewhere = { ...FIX, lines: [35] };
	const rows = rowsOf([{ path: 'a.rs', lines, blocks: blocksOf(lines, 'changes'), fixes: [elsewhere] }]);
	expect(rows.some((row) => row.kind === 'head' && row.tone === 'conflict')).toBe(false);
});

it('never puts a fix and the author’s own change in one card, however close', () => {
	// Edits at 10 and 16: close enough to be one part, but 16 is a fix.
	const lines: DiffLine[] = [];
	for (let n = 1; n <= 30; n++) {
		if (n === 10 || n === 16) {
			lines.push({ origin: 'removed', old: n, new: null, text: `old ${n}` });
			lines.push({ origin: 'added', old: null, new: n, text: `new ${n}` });
		} else {
			lines.push({ origin: 'context', old: n, new: n, text: `line ${n}` });
		}
	}
	expect(blocksOf(lines, 'changes').filter((block) => block.kind === 'hunk')).toHaveLength(1);

	const split = blocksOf(lines, 'changes', new Set(), new Set([16]));
	const hunks = split.filter((block) => block.kind === 'hunk');
	expect(hunks).toHaveLength(2);
	// Back to back: the context between them is shared out, not folded.
	expect(hunks[0].to).toBe(hunks[1].from);

	const rows = rowsOf([{ path: 'a.rs', lines, blocks: split, fixes: [{ ...FIX, lines: [16] }] }]);
	const tones = rows.filter((row) => row.kind === 'head').map((row) => row.kind === 'head' && row.tone);
	expect(tones).toEqual(['hunk', 'conflict']);
});
