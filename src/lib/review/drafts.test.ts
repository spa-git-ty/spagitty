// SPDX-License-Identifier: GPL-3.0-or-later
import { expect, it } from 'vitest';
import type { DiffLine } from '../types';
import { draftsByPlace, pendingOn, placeAt, placeDraft, toDraft, whereOfDraft } from './drafts';
import { normalise } from './record';
import { placeOf } from './threads';

// 1 a / −2 b / +2 B / +3 C / 3 d — line 2 replaced by two.
const LINES: DiffLine[] = [
	{ origin: 'context', old: 1, new: 1, text: 'a' },
	{ origin: 'removed', old: 2, new: null, text: 'b' },
	{ origin: 'added', old: null, new: 2, text: 'B' },
	{ origin: 'added', old: null, new: 3, text: 'C' },
	{ origin: 'context', old: 3, new: 4, text: 'd' }
];

it('counts a line on both sides as GitLab does: the other side’s next number', () => {
	expect(placeAt(LINES, 0)).toEqual({ kind: 'context', old: 1, new: 1 });
	expect(placeAt(LINES, 1)).toEqual({ kind: 'removed', old: 2, new: 2 });
	expect(placeAt(LINES, 2)).toEqual({ kind: 'added', old: 3, new: 2 });
	expect(placeAt(LINES, 3)).toEqual({ kind: 'added', old: 3, new: 3 });
	expect(placeAt(LINES, 4)).toEqual({ kind: 'context', old: 3, new: 4 });
});

it('names a comment on one line by its side and number', () => {
	const on = pendingOn('src/a.rs', null, LINES, 1, 1, '  Why remove this?  ', 'h1', 100);
	expect(on).toMatchObject({
		path: 'src/a.rs',
		line: 2,
		side: 'LEFT',
		startLine: null,
		startSide: null,
		body: 'Why remove this?',
		headSha: 'h1',
		createdAt: 100,
		place: { kind: 'removed', old: 2, new: 2 },
		startPlace: null
	});
});

it('names a range by both ends, across the two sides if it spans them', () => {
	// Picked bottom first: the order of the clicks does not matter.
	const range = pendingOn('src/a.rs', 'src/old.rs', LINES, 3, 1, 'These three', 'h1', 100);
	expect(range).toMatchObject({
		line: 3,
		side: 'RIGHT',
		startLine: 2,
		startSide: 'LEFT',
		place: { kind: 'added', old: 3, new: 3 },
		startPlace: { kind: 'removed', old: 2, new: 2 },
		oldPath: 'src/old.rs'
	});
	expect(toDraft(range)).toEqual({
		path: 'src/a.rs',
		line: 3,
		side: 'RIGHT',
		body: 'These three',
		startLine: 2,
		startSide: 'LEFT',
		place: range.place,
		startPlace: range.startPlace,
		oldPath: 'src/old.rs'
	});
	expect(whereOfDraft(range)).toBe('a.rs:3');
	expect(whereOfDraft(pendingOn('src/a.rs', null, LINES, 2, 3, 'x', 'h1'))).toBe('a.rs:2–3');
});

it('places pending comments where threads are placed', () => {
	const on = pendingOn('src/a.rs', null, LINES, 4, 4, 'x', 'h1');
	expect([...draftsByPlace([on]).keys()]).toEqual([placeOf('src/a.rs', 'RIGHT', 4)]);
});

it('keeps a pending comment whole through the record, places and all', () => {
	const range = pendingOn('src/a.rs', null, LINES, 1, 3, 'These', 'h1', 100);
	const back = normalise({ drafts: [range] }).drafts[0];
	expect(back).toEqual(range);
	// A bad place costs only itself.
	const mangled = normalise({ drafts: [{ ...range, place: { kind: 'sideways', old: 1, new: 1 } }] }).drafts[0];
	expect(mangled.place).toBeNull();
	expect(mangled.body).toBe('These');
});

it('places a draft written by line, with both ends of its range, or says it cannot', () => {
	const draft = {
		...pendingOn('src/a.rs', null, LINES, 2, 3, 'Two new lines.', 'h1', 100),
		place: null,
		startPlace: null
	};
	expect(placeDraft(draft, LINES, 'src/old.rs')).toMatchObject({
		line: 3,
		startLine: 2,
		place: { kind: 'added', old: 3, new: 3 },
		startPlace: { kind: 'added', old: 3, new: 2 },
		oldPath: 'src/old.rs'
	});
	expect(placeDraft({ ...draft, line: 40 }, LINES, null)).toBeNull();
});
