// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { BRANCH, BRANCH_FACTS, entry, FILE, FOLDER, LINE, NOW, TODAY } from '../../testing/life-fixtures';
import {
	AUTHOR_ROWS,
	BIGGEST_EDITS,
	byOf,
	dayOf,
	factsOf,
	filesOf,
	inkOf,
	isOwn,
	KIND_INK,
	layout,
	listed,
	sizesOf,
	targetLabel
} from './chart';

describe('size over time', () => {
	it('is a file’s lines after each commit, as they were counted', () => {
		expect(sizesOf('file', FILE)).toEqual([21, 16, 37, 37, 37, 18]);
	});

	it('keeps the size before a commit whose own could not be read', () => {
		const binary = [entry(3, 1, 'M', 'edit', 'c', 0, 0, { size: 12 }), entry(2, 2, 'M', 'edit', 'b'), entry(1, 3, 'M', 'born', 'a', 0, 0, { size: 9 })];

		expect(sizesOf('file', binary)).toEqual([12, 9, 9]);
	});

	it('is a folder’s lines and files, summed from the oldest entry up', () => {
		const folder = [
			entry(3, 1, 'M', 'removed', 'c', 0, 40, { filesRemoved: 1 }),
			entry(2, 2, 'M', 'edit', 'b', 25, 5),
			entry(1, 3, 'M', 'born', 'a', 100, 0, { filesAdded: 4 })
		];

		expect(sizesOf('folder', folder)).toEqual([80, 120, 100]);
		expect(filesOf(folder)).toEqual([3, 4, 4]);
	});

	it('never goes below nothing when the oldest entries are not read yet', () => {
		const partial = [entry(2, 1, 'M', 'edit', 'b', 3, 0), entry(1, 2, 'M', 'removed', 'a', 0, 500, { filesRemoved: 9 })];

		expect(sizesOf('folder', partial)).toEqual([3, 0]);
		expect(filesOf(partial)).toEqual([0, 0]);
	});

	it('is a branch’s own commits so far, and not what was done to it', () => {
		// merged, commit, rebase, merge, commit, born (the fork: main's commit).
		expect(sizesOf('branch', BRANCH)).toEqual([3, 3, 2, 2, 1, 0]);
	});

	it('is nothing for a line', () => {
		expect(sizesOf('line', LINE)).toEqual([]);
	});
});

describe('what is a commit of the thing itself', () => {
	it('is not where a branch landed, what its reflog says, or the commit it forked from', () => {
		expect(BRANCH.map(isOwn)).toEqual([false, true, false, true, true, false]);
	});

	it('is every entry of a file, and the root a default branch was born at', () => {
		expect(FILE.every(isOwn)).toBe(true);
		expect(isOwn(entry(1, 9, 'M', 'born', 'Initial commit'))).toBe(true);
	});
});

describe('the events a folder lists', () => {
	it('are the ones that added, removed or renamed a file, its birth, and its biggest edits', () => {
		const shown = listed('folder', FOLDER);
		const edits = shown.filter((row) => row.kind === 'edit');

		expect(shown.filter((row) => row.kind !== 'edit').map((row) => row.kind)).toEqual([
			'added',
			'removed',
			'renamed',
			'born'
		]);
		expect(edits).toHaveLength(BIGGEST_EDITS);
		// The thirty edits grow from 1 line to 30: the biggest twenty are 11 to 30.
		expect(Math.min(...edits.map((row) => row.added))).toBe(11);
		// Still newest first.
		expect(shown.map((row) => row.time)).toEqual([...shown.map((row) => row.time)].sort((a, b) => b - a));
	});

	it('never lists an edit that changed no line', () => {
		const quiet = [entry(2, 1, 'M', 'edit', 'mode only'), entry(1, 2, 'M', 'born', 'a', 1, 0)];

		expect(listed('folder', quiet).map((row) => row.summary)).toEqual(['a']);
	});

	it('are every entry, for anything but a folder', () => {
		expect(listed('file', FILE)).toEqual(FILE);
		expect(listed('branch', BRANCH)).toEqual(BRANCH);
		expect(listed('line', LINE)).toEqual(LINE);
	});
});

describe('the facts', () => {
	it('say when a file was born and by whom, and count its commits, authors and renames', () => {
		const facts = factsOf('file', FILE, true, TODAY, null);

		expect(facts.born?.summary).toBe('Add avatar cache');
		expect(facts.commits).toBe(6);
		// Mahmoud, Omar, Sara, and the agent, who is not Mahmoud.
		expect(facts.authors).toBe(4);
		expect(facts.renames).toBe(2);
		expect(facts.now).toBe(21);
		expect(facts.peak).toBe(37);
		expect(facts.unit).toBe('lines');
	});

	it('do not say where it was born until the oldest entry has been read', () => {
		expect(factsOf('file', FILE.slice(0, 3), false, null, null).born).toBeNull();
		// Everything in a range read, and its birth not in it.
		expect(factsOf('file', FILE.slice(0, 3), true, null, null).born).toBeNull();
	});

	it('take the size today from the last commit until the blame is in', () => {
		expect(factsOf('file', FILE, true, null, null).now).toBe(21);
		expect(factsOf('file', FILE, true, { ...TODAY, lines: 22 }, null).now).toBe(22);
	});

	it('count a folder’s files', () => {
		const facts = factsOf('folder', FOLDER, true, null, null);

		expect(facts.files).toBe(13);
		expect(facts.born?.summary).toBe('Start the core crate');
		expect(facts.commits).toBe(FOLDER.length);
	});

	it('count a branch’s own commits and say how it stands against the other branch', () => {
		const facts = factsOf('branch', BRANCH, true, null, BRANCH_FACTS);

		expect(facts.commits).toBe(3);
		expect(facts.authors).toBe(3);
		expect(facts.born?.note).toBe('from main');
		expect(facts.unit).toBe('commits');
		expect(facts).toMatchObject({ against: 'main', ahead: 0, behind: 3 });
	});

	it('measure the default branch against nothing', () => {
		const facts = factsOf('branch', BRANCH, true, null, { ...BRANCH_FACTS, against: null });

		expect(facts.against).toBeNull();
		expect(facts.ahead).toBeNull();
	});

	it('have no size for a line', () => {
		const facts = factsOf('line', LINE, true, TODAY, null);

		expect(facts.now).toBeNull();
		expect(facts.unit).toBeNull();
		expect(facts.born?.summary).toBe('Add avatar cache');
	});
});

describe('the layout', () => {
	const drawn = layout('file', FILE, 1000, 120);

	it('runs time left to right, from the oldest entry to the newest', () => {
		expect(drawn.line.startsWith('M0 ')).toBe(true);
		expect(drawn.line).toContain('L1000 ');
		// One point for each entry.
		expect(drawn.line.split(/[ML]/).filter(Boolean)).toHaveLength(FILE.length);
	});

	it('closes the area along the bottom', () => {
		expect(drawn.area.startsWith(drawn.line)).toBe(true);
		expect(drawn.area.endsWith('L1000 120 L0 120 Z')).toBe(true);
	});

	it('draws the peak near the top and never on the frame', () => {
		const ys = [...drawn.line.matchAll(/[ML][\d.]+ ([\d.]+)/g)].map((match) => Number(match[1]));

		expect(Math.min(...ys)).toBe(12);
		expect(Math.max(...ys)).toBeLessThanOrEqual(120);
	});

	it('marks born, renamed, moved and rewrite, oldest first, and not an ordinary edit', () => {
		expect(drawn.marks.map((mark) => mark.kind)).toEqual(['born', 'renamed', 'moved', 'rewrite']);
		expect(drawn.marks[0].x).toBe(0);
		expect(drawn.marks.map((mark) => mark.x)).toEqual([...drawn.marks.map((mark) => mark.x)].sort());
		expect(drawn.marks[0].ink).toBe(KIND_INK.born);
	});

	it('says how much a rewrite took, on its marker', () => {
		expect(drawn.marks[3].label).toBe('rewrite −81%');
		expect(drawn.marks[1].label).toBe('renamed');
	});

	it('gives each author a row of ticks, in the order they first appear', () => {
		expect(drawn.rows.map((row) => row.name)).toEqual(['Mahmoud', 'Sara', 'Omar', 'Codex']);
		expect(drawn.rows[0].ticks).toHaveLength(3);
		expect(drawn.rows[3].ticks).toEqual([1]);
	});

	/** Agent identity is unchanged from 2.0: the agent's colour. */
	it('draws an agent in the agent’s colour and a person in their own', () => {
		const codex = drawn.rows.find((row) => row.name === 'Codex')!;
		const mahmoud = drawn.rows.find((row) => row.name === 'Mahmoud')!;

		expect(codex.agent).toBe(true);
		expect(codex.ink).toBe('var(--agent)');
		expect(mahmoud.agent).toBe(false);
		expect(mahmoud.ink).toMatch(/^var\(--lane-\d\)$/);
		expect(mahmoud.ink).not.toBe('var(--agent)');
	});

	it('counts the authors past the rows there is room for', () => {
		const many = Array.from({ length: AUTHOR_ROWS + 3 }, (_, i) =>
			entry(i + 1, AUTHOR_ROWS + 3 - i, 'M', 'edit', 'x', 1, 0, {
				size: 5,
				authorName: `Person ${i}`,
				authorEmail: `p${i}@example.com`
			})
		).reverse();
		const crowded = layout('file', many);

		expect(crowded.rows).toHaveLength(AUTHOR_ROWS);
		expect(crowded.others).toBe(3);
	});

	it('labels the bottom by month for a life of months, and by day for one of days', () => {
		expect(drawn.axis).toHaveLength(8);
		expect(drawn.axis.every((label) => /^[A-Z][a-z]{2}$/.test(label))).toBe(true);
		expect(layout('branch', BRANCH).axis.every((label) => /^\d+ [A-Z][a-z]{2}$/.test(label))).toBe(true);

		const years = [entry(2, 0, 'M', 'edit', 'b', 0, 0, { size: 2 }), entry(1, 1500, 'M', 'born', 'a', 0, 0, { size: 1 })];
		expect(layout('file', years).axis.every((label) => /^\d{4}$/.test(label))).toBe(true);
	});

	it('draws a life of one commit in the middle, as a level line', () => {
		const one = layout('file', [entry(1, 5, 'M', 'born', 'a', 3, 0, { size: 3 })]);

		expect(one.marks).toEqual([{ x: 0.5, kind: 'born', label: 'born', ink: KIND_INK.born }]);
		expect(one.rows[0].ticks).toEqual([0.5]);
		expect(one.line.startsWith('M0 ')).toBe(true);
		expect(one.axis).toHaveLength(1);
	});

	it('has no line where there is no size, and still has the authors', () => {
		const line = layout('line', LINE);

		expect(line.line).toBe('');
		expect(line.area).toBe('');
		expect(line.rows.map((row) => row.name)).toEqual(['Mahmoud', 'Omar', 'Codex']);
	});

	it('leaves what was done to a branch out of its authors’ rows', () => {
		const branch = layout('branch', BRANCH);

		// The merge of main and the fork commit are Mahmoud's; only the merge is the branch's.
		expect(branch.rows.map((row) => [row.name, row.ticks.length])).toEqual([
			['Omar', 1],
			['Mahmoud', 1],
			['Codex', 1]
		]);
		expect(branch.marks.map((mark) => mark.kind)).toEqual(['born', 'merged']);
	});

	it('says what the chart shows, for a reader who cannot see it', () => {
		expect(drawn.said).toBe('Lines over time: 18 at first, 37 at most, 21 now');
		expect(layout('branch', BRANCH).said).toContain('Commits over time');
	});

	it('is empty for no entries', () => {
		expect(layout('file', [])).toEqual({ line: '', area: '', marks: [], rows: [], others: 0, axis: [], said: '' });
	});
});

describe('labels', () => {
	it('names who an entry is by: the agent, or the author', () => {
		expect(byOf(FILE[0])).toBe('Codex');
		expect(byOf(FILE[1])).toBe('Omar');
		expect(inkOf(FILE[0])).toBe('var(--agent)');
	});

	it('writes a day with its year only when it is not this one', () => {
		const now = NOW * 1000;

		expect(dayOf(NOW - 86_400 * 400, now)).toMatch(/^\d+ [A-Z][a-z]{2} \d{4}$/);
		expect(dayOf(NOW + 86_400 * 20, now)).toMatch(/^\d+ [A-Z][a-z]{2}$/);
	});

	it('writes a target as it is typed', () => {
		expect(targetLabel({ kind: 'file', path: 'src/a.rs' })).toBe('src/a.rs');
		expect(targetLabel({ kind: 'line', path: 'src/a.rs', line: 14 })).toBe('src/a.rs:14');
		expect(targetLabel({ kind: 'branch', name: 'feature/x' })).toBe('feature/x');
		expect(targetLabel({ kind: 'folder', path: '' })).toBe('/');
	});

	it('has a colour for every kind', () => {
		for (const kind of ['born', 'renamed', 'moved', 'rewrite', 'edit', 'deleted', 'added', 'removed', 'commit', 'merge', 'merged', 'rebase', 'reset'] as const) {
			expect(KIND_INK[kind]).toMatch(/^var\(--/);
		}
	});
});
