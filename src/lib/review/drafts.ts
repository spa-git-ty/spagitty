// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Comments written in the review room and not yet sent (FEAT-093).
 *
 * A pending comment is kept in the pull request's record, so it survives a
 * restart, and goes out only with Finish review. It names its lines the way
 * both hosts need: GitHub by a side and a number at each end, GitLab by both
 * versions' counters (`LinePlace`). Both are worked out here, once, from the
 * whole file the comment was written on.
 */

import type { DraftComment, DiffLine, LinePlace } from '../types';
import type { PendingComment } from './record';
import { placeOf } from './threads';

/**
 * Where line `index` of a whole file sits by both versions' counters: its own
 * number on its side, and on the other the number the next line there has.
 */
export function placeAt(lines: DiffLine[], index: number): LinePlace {
	let old = 0;
	let fresh = 0;
	for (let i = 0; i < index; i++) {
		old = lines[i].old ?? old;
		fresh = lines[i].new ?? fresh;
	}
	const line = lines[index];
	if (line.origin === 'added') return { kind: 'added', old: old + 1, new: line.new ?? fresh + 1 };
	if (line.origin === 'removed') return { kind: 'removed', old: line.old ?? old + 1, new: fresh + 1 };
	return { kind: 'context', old: line.old ?? old + 1, new: line.new ?? fresh + 1 };
}

/** A line's side and number as GitHub names them: a removed line by its old one. */
export function sideOf(line: DiffLine): { side: 'LEFT' | 'RIGHT'; line: number } {
	return line.origin === 'removed' ? { side: 'LEFT', line: line.old ?? 0 } : { side: 'RIGHT', line: line.new ?? 0 };
}

/** A pending comment on lines `from` to `to` of a whole file. */
export function pendingOn(
	path: string,
	oldPath: string | null,
	lines: DiffLine[],
	from: number,
	to: number,
	body: string,
	headSha: string,
	now = Math.floor(Date.now() / 1000)
): PendingComment {
	const [first, last] = from <= to ? [from, to] : [to, from];
	const end = sideOf(lines[last]);
	const start = first < last ? sideOf(lines[first]) : null;
	return {
		id: `${path}:${end.side}:${end.line}:${now}:${Math.random().toString(36).slice(2, 8)}`,
		path,
		line: end.line,
		side: end.side,
		startLine: start?.line ?? null,
		startSide: start?.side ?? null,
		body: body.trim(),
		headSha,
		createdAt: now,
		place: placeAt(lines, last),
		startPlace: first < last ? placeAt(lines, first) : null,
		oldPath
	};
}

/** What the backend sends for a pending comment. */
export function toDraft(pending: PendingComment): DraftComment {
	return {
		path: pending.path,
		line: pending.line,
		side: pending.side,
		body: pending.body,
		startLine: pending.startLine,
		startSide: pending.startSide,
		place: pending.place,
		startPlace: pending.startPlace,
		oldPath: pending.oldPath
	};
}

/** Pending comments by where they sit, as threads are. */
export function draftsByPlace(drafts: PendingComment[]): Map<string, PendingComment[]> {
	const out = new Map<string, PendingComment[]>();
	for (const draft of drafts) {
		const place = placeOf(draft.path, draft.side, draft.line);
		const list = out.get(place);
		if (list) list.push(draft);
		else out.set(place, [draft]);
	}
	return out;
}

/** `name.rs:17`, or `name.rs:12–17` for a range on one side. */
export function whereOfDraft(draft: PendingComment): string {
	const name = draft.path.slice(draft.path.lastIndexOf('/') + 1);
	if (draft.startLine !== null && draft.startSide === draft.side) return `${name}:${draft.startLine}–${draft.line}`;
	return `${name}:${draft.line}`;
}
