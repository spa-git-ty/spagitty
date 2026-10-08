// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { changes, type Wants } from './changes';
import type { Watched } from '$lib/types';

const ALL: Wants = {
	notifyMerged: true,
	notifyComments: true,
	notifyReviewRequests: true,
	notifyChecks: true
};

function pr(overrides: Partial<Watched> = {}): Watched {
	return {
		key: 'github.com/o/r#7',
		host: 'github.com',
		repository: 'o/r',
		number: 7,
		title: 'Fix the thing',
		url: 'https://github.com/o/r/pull/7',
		state: 'open',
		mine: true,
		reviewRequested: false,
		activity: 0,
		lastActor: null,
		lastActorIsMe: false,
		checks: null,
		...overrides
	};
}

function seen(...rows: Watched[]): Record<string, Watched> {
	return Object.fromEntries(rows.map((row) => [row.key, row]));
}

describe('changes', () => {
	it('announces nothing on the first read', () => {
		// Turning notifications on must not open with everything that ever happened.
		expect(changes(null, [pr({ state: 'merged' })], ALL)).toEqual([]);
	});

	it('says when my pull request was merged, and which one', () => {
		const [news] = changes(seen(pr()), [pr({ state: 'merged' })], ALL);

		expect(news.kind).toBe('merged');
		expect(news.title).toBe('Your pull request was merged');
		expect(news.body).toBe('Fix the thing · o/r#7');
	});

	it('says closed rather than merged when it was closed', () => {
		expect(changes(seen(pr()), [pr({ state: 'closed' })], ALL)[0].kind).toBe('closed');
	});

	it('does not announce somebody else being merged', () => {
		expect(changes(seen(pr({ mine: false })), [pr({ mine: false, state: 'merged' })], ALL)).toEqual([]);
	});

	it('names who commented', () => {
		const [news] = changes(seen(pr()), [pr({ activity: 2, lastActor: 'ada' })], ALL);

		expect(news.kind).toBe('comment');
		expect(news.title).toBe('ada commented');
	});

	it('stays quiet about my own comment', () => {
		const now = [pr({ activity: 1, lastActor: 'me', lastActorIsMe: true })];

		expect(changes(seen(pr()), now, ALL)).toEqual([]);
	});

	it('announces a new review request, including on a pull request seen for the first time', () => {
		const asked = pr({ key: 'github.com/o/r#9', number: 9, mine: false, reviewRequested: true });

		expect(changes(seen(pr()), [pr(), asked], ALL).map((n) => n.kind)).toEqual(['reviewRequested']);
		expect(changes(seen(pr({ ...asked, reviewRequested: false })), [asked], ALL)[0].kind).toBe(
			'reviewRequested'
		);
	});

	it('announces checks only when they start failing', () => {
		expect(changes(seen(pr({ checks: 'running' })), [pr({ checks: 'failing' })], ALL)[0].kind).toBe(
			'checksFailed'
		);
		expect(changes(seen(pr({ checks: 'failing' })), [pr({ checks: 'failing' })], ALL)).toEqual([]);
	});

	it('leaves out each kind that is switched off', () => {
		const off: Wants = {
			notifyMerged: false,
			notifyComments: false,
			notifyReviewRequests: false,
			notifyChecks: false
		};
		const before = seen(pr({ checks: 'running' }));
		const now = [pr({ activity: 3, lastActor: 'ada', checks: 'failing', reviewRequested: true })];

		expect(changes(before, now, off)).toEqual([]);
		expect(changes(seen(pr()), [pr({ state: 'merged' })], off)).toEqual([]);
	});
});
