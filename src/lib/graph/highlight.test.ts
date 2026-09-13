// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';

import { branchOf, byAuthor } from './highlight';
import type { GraphRow, RefChip } from '../types';

/**
 * A row, with only the fields these functions look at spelled out.
 *
 * The graph's rows are wide and none of this code reads the geometry, so
 * writing them out in full would bury the one thing each test is about.
 */
function makeRow(
	index: number,
	id: string,
	parents: string[],
	extra: { author?: string; refs?: string[]; lane?: number; kind?: RefChip['kind']; current?: boolean } = {}
): GraphRow {
	const refs: RefChip[] = (extra.refs ?? []).map((name) => ({
		name,
		kind: extra.kind ?? 'branch',
		current: extra.current ?? false,
		local: true,
		remotes: [], divergence: null
	}));

	return {
		index,
		id,
		short: id.slice(0, 7),
		summary: `commit ${id}`,
		authorName: extra.author ?? 'Ada Lovelace',
		authorEmail: `${(extra.author ?? 'ada').toLowerCase().replace(/\s+/g, '.')}@example.com`,
		initials: 'AL',
		time: 1_700_000_000 - index * 60,
		lane: extra.lane ?? 0,
		color: 0,
		signed: false,
		parents,
		refs,
		edges: []
	};
}

/** Rows newest-first, the order the graph holds them in. */
function accessor(rows: GraphRow[]) {
	return (index: number) => rows[index];
}

/**
 * A history with a topic branch merged back:
 *
 *   0  m2   merge, parents m1 and t1     [main]
 *   1  t1   topic commit                 [topic]
 *   2  m1   main commit
 *   3  b0   the base both descend from
 *   4  root
 */
const MERGED: GraphRow[] = [
	makeRow(0, 'm2', ['m1', 't1'], { refs: ['main'] }),
	makeRow(1, 't1', ['b0'], { author: 'Grace Hopper', refs: ['topic'] }),
	makeRow(2, 'm1', ['b0']),
	makeRow(3, 'b0', ['root']),
	makeRow(4, 'root', [])
];

describe('byAuthor', () => {
	const rows = accessor(MERGED);
	const last = MERGED.length - 1;

	it('dims nothing when there is no filter', () => {
		expect(byAuthor('', rows, 0, last)).toBeNull();
		expect(byAuthor('   ', rows, 0, last)).toBeNull();
	});

	it('matches on a substring, so a surname finds a full name', () => {
		expect(byAuthor('hopper', rows, 0, last)).toEqual(new Set([1]));
	});

	it('ignores case and surrounding space', () => {
		expect(byAuthor('  HOPPER ', rows, 0, last)).toEqual(new Set([1]));
	});

	it('returns an empty set — not null — when a real filter matches nothing', () => {
		// The distinction matters: null means "dim nothing", empty means
		// "nothing matched", and the two must not render the same.
		expect(byAuthor('nobody', rows, 0, last)).toEqual(new Set());
	});

	it('only considers the range it was given', () => {
		expect(byAuthor('hopper', rows, 2, last)).toEqual(new Set());
	});

	it('skips rows the walk has not delivered', () => {
		const sparse = [undefined as unknown as GraphRow, MERGED[1]];
		expect(byAuthor('hopper', accessor(sparse), 0, 1)).toEqual(new Set([1]));
	});
});

describe('branchOf', () => {
	it('names nothing for a row that shows its own branch', () => {
		expect(branchOf(0, accessor(MERGED))).toBeNull();
		expect(branchOf(1, accessor(MERGED))).toBeNull();
	});

	it('follows first-parent children up to the branch whose history it is', () => {
		expect(branchOf(2, accessor(MERGED))).toBe('main');
	});

	/**
	 *   0  m2  merge of m1 and t2   [main]   lane 0
	 *   1  t2  topic                [topic]  lane 1
	 *   2  t1  topic                         lane 1
	 *   3  m1  main                          lane 0
	 *   4  b0  base, forks here              lane 0
	 */
	const FORK: GraphRow[] = [
		makeRow(0, 'm2', ['m1', 't2'], { refs: ['main'] }),
		makeRow(1, 't2', ['t1'], { refs: ['topic'], lane: 1 }),
		makeRow(2, 't1', ['b0'], { lane: 1 }),
		makeRow(3, 'm1', ['b0']),
		makeRow(4, 'b0', ['root']),
		makeRow(5, 'root', [])
	];

	it('names a topic commit for its topic, not for the branch that merged it', () => {
		expect(branchOf(2, accessor(FORK))).toBe('topic');
	});

	it('at a fork, prefers the child drawn in the same lane', () => {
		// t1 is the nearer first-parent child but sits in lane 1; m1 continues
		// lane 0, which is the line the graph draws through b0.
		expect(branchOf(4, accessor(FORK))).toBe('main');
	});

	it('falls back to a first-parent child in another lane', () => {
		const rows = [
			makeRow(0, 'tip', ['a'], { refs: ['feature'], lane: 2 }),
			makeRow(1, 'a', ['base'], { lane: 2 }),
			makeRow(2, 'base', [], { lane: 0 })
		];
		expect(branchOf(2, accessor(rows))).toBe('feature');
	});

	it('climbs through a merge when the merged branch is gone', () => {
		const rows = [
			makeRow(0, 'm2', ['m1'], { refs: ['main'] }),
			makeRow(1, 'm1', ['m0', 'gone'], {}),
			makeRow(2, 'gone', ['m0'], { lane: 1 }),
			makeRow(3, 'm0', [])
		];
		expect(branchOf(2, accessor(rows))).toBe('main');
	});

	it('ignores tags and prefers a local branch over a remote one', () => {
		const rows = [
			makeRow(0, 'r', ['t'], { refs: ['origin/main'], kind: 'remote' }),
			makeRow(1, 't', ['c'], { refs: ['v1.0'], kind: 'tag' }),
			makeRow(2, 'c', [])
		];
		expect(branchOf(2, accessor(rows))).toBe('origin/main');

		const both: GraphRow = {
			...makeRow(0, 'x', ['c'], { refs: ['origin/dev'], kind: 'remote' }),
			refs: [
				{ name: 'origin/dev', kind: 'remote', current: false, local: false, remotes: [], divergence: null },
				{ name: 'dev', kind: 'branch', current: false, local: true, remotes: [], divergence: null }
			]
		};
		expect(branchOf(1, accessor([both, makeRow(1, 'c', [])]))).toBe('dev');
	});

	it('says nothing when no branch is above it in the loaded rows', () => {
		const rows = [makeRow(0, 'a', ['b']), makeRow(1, 'b', [])];
		expect(branchOf(1, accessor(rows))).toBeNull();
		expect(branchOf(5, accessor(rows))).toBeNull();
	});

	it('gives up on a history whose tip is beyond the child window', () => {
		const rows: GraphRow[] = [makeRow(0, 'tip', ['old'], { refs: ['far'] })];
		for (let i = 1; i < 700; i++) rows.push(makeRow(i, `n${i}`, [`n${i + 1}`], { lane: 1 }));
		rows.push(makeRow(700, 'old', []));
		expect(branchOf(700, accessor(rows))).toBeNull();
	});
});
