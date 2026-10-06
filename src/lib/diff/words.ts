// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The words that changed inside a changed line (FEAT-090).
 *
 * A line that went from `if let Some(hit) = MEMORY.lock().get(&key) {` to
 * `if path.exists() && !is_stale(&path, MAX_AGE)? {` is two whole rows of
 * colour in a plain diff, and the eye has to find what differs by itself. This
 * pairs each removed line with the added line that replaced it and marks only
 * the words between them that differ.
 *
 * Pure, and bounded: lines are compared word by word with a longest common
 * subsequence, which is quadratic, so a pair too long to compare cheaply is
 * left unmarked rather than slowing the pane down.
 */

import type { DiffLine } from '../types';

export interface Segment {
	text: string;
	changed: boolean;
}

/** Past this many words on either side, a pair is not compared. */
const MAX_TOKENS = 400;

/** Words, runs of space, and single punctuation marks. */
export function tokens(text: string): string[] {
	return text.match(/[\p{L}\p{N}_]+|\s+|[^\p{L}\p{N}_\s]/gu) ?? [];
}

/**
 * Which tokens of `a` and of `b` are not in their longest common subsequence.
 * Returned as segments, with neighbours of the same kind merged.
 */
export function diffWords(a: string, b: string): [Segment[], Segment[]] | null {
	const x = tokens(a);
	const y = tokens(b);
	if (x.length === 0 || y.length === 0) return null;
	if (x.length > MAX_TOKENS || y.length > MAX_TOKENS) return null;

	// lcs[i][j]: the common length of x[i..] and y[j..].
	const lcs: Uint16Array[] = Array.from({ length: x.length + 1 }, () => new Uint16Array(y.length + 1));
	for (let i = x.length - 1; i >= 0; i--) {
		for (let j = y.length - 1; j >= 0; j--) {
			lcs[i][j] = x[i] === y[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
		}
	}

	const left: Segment[] = [];
	const right: Segment[] = [];
	const push = (list: Segment[], text: string, changed: boolean) => {
		const last = list[list.length - 1];
		if (last && last.changed === changed) last.text += text;
		else list.push({ text, changed });
	};

	let i = 0;
	let j = 0;
	let same = 0;
	while (i < x.length && j < y.length) {
		if (x[i] === y[j]) {
			if (/\S/.test(x[i])) same += 1;
			push(left, x[i++], false);
			push(right, y[j++], false);
		} else if (lcs[i + 1][j] >= lcs[i][j + 1]) {
			push(left, x[i++], true);
		} else {
			push(right, y[j++], true);
		}
	}
	while (i < x.length) push(left, x[i++], true);
	while (j < y.length) push(right, y[j++], true);

	// Too little kept to call it an edit: the line was rewritten, and marking
	// most of it would say no more than the row's own colour does.
	const meaningful = (list: string[]) => list.filter((token) => /\S/.test(token)).length;
	if (same === 0 || same < Math.min(meaningful(x), meaningful(y)) / 3) return null;

	return [settle(left), settle(right)];
}

/** Neighbours of the same kind as one segment, empty ones dropped. */
function merge(segments: Segment[]): Segment[] {
	const out: Segment[] = [];
	for (const segment of segments) {
		if (segment.text === '') continue;
		const last = out[out.length - 1];
		if (last && last.changed === segment.changed) last.text += segment.text;
		else out.push({ ...segment });
	}
	return out;
}

/**
 * Shape the marks the way a reader sees an edit.
 *
 * - Space or punctuation alone between two changed words is part of the
 *   change: marking `foo` and `bar` but not the `.` or space between them
 *   reads as two edits, and a bracket that happened to match is not news.
 * - A mark does not start or end with space: the highlight sits on the words.
 */
function settle(segments: Segment[]): Segment[] {
	const bridged = segments.map((segment, k) => {
		const island =
			!segment.changed &&
			!/[\p{L}\p{N}_]/u.test(segment.text) &&
			segments[k - 1]?.changed &&
			segments[k + 1]?.changed;
		return island ? { text: segment.text, changed: true } : segment;
	});

	const trimmed: Segment[] = [];
	for (const segment of merge(bridged)) {
		if (!segment.changed) {
			trimmed.push(segment);
			continue;
		}
		const [, lead, core, tail] = segment.text.match(/^(\s*)([\s\S]*?)(\s*)$/) ?? ['', '', segment.text, ''];
		trimmed.push({ text: lead, changed: false });
		trimmed.push({ text: core, changed: true });
		trimmed.push({ text: tail, changed: false });
	}
	return merge(trimmed);
}

/**
 * How good a pairing two lines make, higher is better.
 *
 * How much of the shorter line they share, less a little for every mark past
 * one on each side: one contiguous edit reads as an edit, where three
 * scattered marks read as noise — so of two candidates, the one that changed
 * in one place wins.
 */
function likeness(a: Segment[], b: Segment[]): number {
	const kept = (list: Segment[]) =>
		list
			.filter((segment) => !segment.changed)
			.map((segment) => segment.text)
			.join('')
			.replace(/\s+/g, '').length;
	const all = (list: Segment[]) => list.map((segment) => segment.text).join('').replace(/\s+/g, '').length;
	const marks = (list: Segment[]) => list.filter((segment) => segment.changed).length;
	const shorter = Math.min(all(a), all(b));
	if (shorter === 0) return 0;
	const scattered = Math.max(0, marks(a) - 1) + Math.max(0, marks(b) - 1);
	return Math.min(kept(a), kept(b)) / shorter - 0.15 * scattered;
}

/** Past this many candidate pairs in one run, lines pair by position. */
const MAX_PAIRS = 64;

/**
 * The marked segments of every changed line that has a partner, by its index
 * in `lines`.
 *
 * Within each run of removed lines followed by added lines, each removed line
 * pairs with the added line after the last one paired that it most resembles
 * — so a line that was replaced by the second of three new lines is compared
 * with that one, not with whatever came first. A run too long to weigh every
 * pair pairs by position. Lines left over stay whole.
 */
export function pairWords(lines: DiffLine[]): Map<number, Segment[]> {
	const marked = new Map<number, Segment[]>();
	let i = 0;
	while (i < lines.length) {
		if (lines[i].origin !== 'removed') {
			i++;
			continue;
		}
		const removed: number[] = [];
		while (i < lines.length && lines[i].origin === 'removed') removed.push(i++);
		const added: number[] = [];
		while (i < lines.length && lines[i].origin === 'added') added.push(i++);

		if (removed.length * added.length > MAX_PAIRS) {
			for (let k = 0; k < Math.min(removed.length, added.length); k++) {
				const pair = diffWords(lines[removed[k]].text, lines[added[k]].text);
				if (!pair) continue;
				marked.set(removed[k], pair[0]);
				marked.set(added[k], pair[1]);
			}
			continue;
		}

		let from = 0;
		for (const r of removed) {
			let best: { at: number; pair: [Segment[], Segment[]]; score: number } | null = null;
			for (let a = from; a < added.length; a++) {
				const pair = diffWords(lines[r].text, lines[added[a]].text);
				if (!pair) continue;
				const score = likeness(pair[0], pair[1]);
				if (!best || score > best.score) best = { at: a, pair, score };
			}
			if (!best) continue;
			marked.set(r, best.pair[0]);
			marked.set(added[best.at], best.pair[1]);
			from = best.at + 1;
		}
	}
	return marked;
}
