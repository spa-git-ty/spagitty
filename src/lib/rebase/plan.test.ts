// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { counts, orphanSquash, sentence } from './plan';
import type { RebaseEdit, RebasePreview } from '../types';

const edits = (...actions: RebaseEdit['action'][]): RebaseEdit[] =>
	actions.map((action, index) => ({ id: `c${index}`, action }));

const preview = (rows: number, risky = 0): RebasePreview => ({
	rows: Array.from({ length: rows }, (_, index) => ({
		id: `r${index}`,
		short: `r${index}`,
		summary: '',
		absorbed: [],
		reworded: false,
		mayConflict: index < risky
	})),
	dropped: [],
	refusal: null,
	emptiesTheBranch: rows === 0
});

describe('counts', () => {
	it('counts each kind of edit, and what the branch has after', () => {
		const c = counts(edits('pick', 'squash', 'reword', 'drop', 'pick'), preview(3, 1));
		expect(c).toEqual({ after: 3, replayed: 4, squashed: 1, reworded: 1, dropped: 1, risky: 1 });
	});

	it('works the result out from the plan before the preview arrives', () => {
		expect(counts(edits('pick', 'squash', 'drop'), null).after).toBe(1);
	});
});

describe('sentence', () => {
	it('says what moves where, and what each edit does', () => {
		const c = counts(edits('pick', 'squash', 'drop'), preview(1));
		expect(sentence('topic', 'main', c)).toBe(
			'2 commits of topic are replayed on top of main, 1 folded into the one above, 1 left out. topic moves to the last of them, with new ids.'
		);
	});

	it('says plainly when nothing is left', () => {
		expect(sentence('topic', 'main', counts(edits('drop'), preview(0)))).toBe(
			'Every commit is dropped: topic ends up exactly at main.'
		);
	});
});

describe('orphanSquash', () => {
	it('flags a squash with nothing above it, past any drops', () => {
		expect(orphanSquash(edits('drop', 'squash', 'pick'), 'c1')).toBe(true);
		expect(orphanSquash(edits('pick', 'squash'), 'c1')).toBe(false);
		expect(orphanSquash(edits('squash'), 'c0')).toBe(true);
	});
});
