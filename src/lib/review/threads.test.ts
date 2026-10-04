// SPDX-License-Identifier: GPL-3.0-or-later
import { expect, it } from 'vitest';
import type { PullRequestComment } from '../types';
import { placeOf, threadsByPlace, threadsOf, whereOf } from './threads';

function comment(id: number, inReplyTo: number | null, createdAt: number, extra: Partial<PullRequestComment> = {}) {
	return {
		id,
		inReplyTo,
		path: 'src/avatars.rs',
		line: 20,
		side: 'RIGHT',
		body: `comment ${id}`,
		author: 'nour.h',
		createdAt,
		resolved: false,
		...extra
	} satisfies PullRequestComment;
}

it('follows a chain of replies back to the comment that started it', () => {
	// GitHub chains a reply to a reply; GitLab names the discussion's first note.
	const threads = threadsOf([
		comment(3, 2, 30),
		comment(1, null, 10),
		comment(2, 1, 20),
		comment(5, 4, 50, { line: 43, side: 'LEFT' }),
		comment(4, null, 40, { line: 43, side: 'LEFT', resolved: true })
	]);
	expect(threads.map((thread) => thread.id)).toEqual([1, 4]);
	expect(threads[0].comments.map((c) => c.id)).toEqual([1, 2, 3]);
	expect(threads[1]).toMatchObject({ side: 'LEFT', line: 43, resolved: true });
});

it('makes a thread of a reply whose parent is missing, and stops on a loop', () => {
	const threads = threadsOf([comment(9, 99, 10), comment(1, 2, 20), comment(2, 1, 30)]);
	expect(threads).toHaveLength(2);
	expect(threads[0].comments.map((c) => c.id)).toEqual([9]);
});

it('places threads by file, side and line, and leaves out ones with no line', () => {
	const threads = threadsOf([comment(1, null, 10), comment(2, null, 20, { line: null })]);
	const places = threadsByPlace(threads);
	expect([...places.keys()]).toEqual([placeOf('src/avatars.rs', 'RIGHT', 20)]);
	expect(whereOf(threads[0])).toBe('avatars.rs:20');
	expect(whereOf(threads[1])).toBe('avatars.rs');
});
