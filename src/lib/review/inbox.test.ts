// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from 'vitest';
import { request } from '../../testing/git-fixtures';
import {
	chipsOf,
	continueLabel,
	factsOf,
	groupInbox,
	inboxOrder,
	progressOf,
	sizeLabel,
	sizeOf
} from './inbox';
import { emptyRecord, type ReviewRecord } from './record';

function record(overrides: Partial<ReviewRecord> = {}): ReviewRecord {
	return { ...emptyRecord(), ...overrides };
}

describe('groupInbox', () => {
	const asked = request({ id: 'a', reviewRequested: true });
	const answered = request({ id: 'b', repliesToYou: 1 });
	const open = request({ id: 'c' });
	const mine = request({ id: 'd', authorName: 'Me', reviewRequested: true });

	it('puts what you were asked to review first, then what came back, then the rest', () => {
		const groups = groupInbox([open, answered, asked], 'me');
		expect(groups.map((group) => group.id)).toEqual(['needs', 'back', 'open']);
		expect(groups.map((group) => group.items.map((pr) => pr.id))).toEqual([['a'], ['b'], ['c']]);
		expect(groups[0].title).toBe('Needs you');
		expect(groups[0].hint).toBe('you are a requested reviewer');
		expect(groups[1].hint).toBe('the author answered you');
	});

	it('puts your own pull requests last, under Yours, whatever case your login is in', () => {
		const groups = groupInbox([mine, open], 'me');
		expect(inboxOrder(groups).map((pr) => pr.id)).toEqual(['c', 'd']);
		expect(groups.at(-1)?.title).toBe('Yours');
		// Requested or not, your own is not in Needs you.
		expect(groups[0].id).toBe('open');
	});

	it('keeps a requested review in Needs you even when it also has replies', () => {
		const both = request({ id: 'e', reviewRequested: true, repliesToYou: 2 });
		const groups = groupInbox([both], 'me');
		expect(groups).toHaveLength(1);
		expect(groups[0].id).toBe('needs');
	});

	it('draws no empty group, and names the last one by scope', () => {
		expect(groupInbox([], 'me')).toEqual([]);
		expect(groupInbox([open], 'me', 'repo')[0].title).toBe('Open on this repo');
		expect(groupInbox([open], 'me', 'all')[0].title).toBe('Involving you');
	});
});

describe('size', () => {
	it('reads one to three bars from the lines changed', () => {
		expect(sizeOf(request({ added: 16, removed: 8 }))).toBe(1);
		expect(sizeOf(request({ added: 240, removed: 61 }))).toBe(2);
		expect(sizeOf(request({ added: 1210, removed: 980 }))).toBe(3);
	});

	it('falls back to the file count when the host sent no line counts', () => {
		expect(sizeOf(request({ added: 0, removed: 0, changedFiles: 3 }))).toBe(1);
		expect(sizeOf(request({ added: 0, removed: 0, changedFiles: 12 }))).toBe(2);
		expect(sizeOf(request({ added: 0, removed: 0, changedFiles: 40 }))).toBe(3);
	});

	it('says files and lines, and only files when that is all it knows', () => {
		expect(sizeLabel(request({ changedFiles: 23, added: 1210, removed: 980 }))).toEqual({
			files: '23 files',
			lines: '+1,210 −980'
		});
		expect(sizeLabel(request({ changedFiles: 1, added: 0, removed: 0 }))).toEqual({
			files: '1 file',
			lines: null
		});
	});
});

describe('progressOf', () => {
	const pr = request({ changedFiles: 6, headSha: 'head-2' });

	it('says not started with nothing ticked', () => {
		expect(progressOf(pr, null)).toEqual({ share: 0, label: 'not started', changed: false });
		expect(progressOf(pr, record()).label).toBe('not started');
	});

	it('counts ticked files against the file count', () => {
		const done = record({ headSha: 'head-2', viewed: { a: '1', b: '2', c: '3' } });
		expect(progressOf(pr, done)).toEqual({ share: 0.5, label: '3 of 6 viewed', changed: false });
		expect(continueLabel(progressOf(pr, done))).toBe('Continue review · 3 of 6 viewed');
		expect(continueLabel(progressOf(pr, null))).toBe('Start review');
	});

	it('says so when the author pushed after you looked', () => {
		const stale = record({ headSha: 'head-1', viewed: { a: '1' } });
		const progress = progressOf(pr, stale);
		expect(progress.changed).toBe(true);
		expect(progress.label).toBe('changed since you looked');
	});
});

describe('chipsOf and factsOf', () => {
	it('names open threads, or replies to you in Back with you', () => {
		const pr = request({ openThreads: 2, repliesToYou: 1 });
		expect(chipsOf(pr, null, 'needs').threads).toBe('2 open threads');
		expect(chipsOf(pr, null, 'back').threads).toBe('1 reply to you');
		expect(chipsOf(request(), null, 'open').threads).toBeNull();
	});

	it('marks conflict fixes only once a look has found them', () => {
		expect(chipsOf(request(), null, 'needs').conflict).toBeNull();
		expect(chipsOf(request(), record({ conflictFiles: ['a.rs'] }), 'needs').conflict).toBe(
			'conflict fixes'
		);
	});

	it('marks a pull request the host cannot merge, and only that (BUG-063)', () => {
		expect(chipsOf(request({ mergeable: false }), null, 'needs').base).toBe('conflicts with main');
		expect(chipsOf(request({ mergeable: null }), null, 'needs').base).toBeNull();
		expect(chipsOf(request({ mergeable: true }), null, 'back').base).toBeNull();
		expect(factsOf(request({ mergeable: false }), null)[0]).toEqual({
			tone: 'danger',
			text: 'Conflicts with main: it cannot be merged as it stands'
		});
		expect(factsOf(request({ mergeable: null }), null)).not.toContainEqual(
			expect.objectContaining({ tone: 'danger' })
		);
	});

	it('lists what is worth knowing before you start, in order', () => {
		const pr = request({ openThreads: 2, resolvedThreads: 1, checks: 'passing' });
		const saved = record({
			conflictFiles: ['a.rs', 'b.ts'],
			conflictMerges: ['7c1e9a0'],
			drafts: [
				{
					id: '1',
					path: 'a.rs',
					line: 17,
					side: 'RIGHT',
					startLine: null,
					startSide: null,
					body: 'Fall through?',
					headSha: 'x',
					createdAt: 0,
					place: null,
					startPlace: null,
					oldPath: null
				}
			]
		});

		expect(factsOf(pr, saved)).toEqual([
			{ tone: 'resolve', text: '2 files carry conflict fixes from merge 7c1e9a0' },
			{ tone: 'accent', text: '2 open threads, 1 resolved' },
			{ tone: 'ok', text: 'Checks passing' },
			{ tone: 'warn', text: '1 pending comment of yours' }
		]);
	});

	it('says nothing it does not know', () => {
		expect(factsOf(request({ checks: null }), null)).toEqual([]);
	});
});
