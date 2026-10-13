// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A life, laid out (FEAT-132).
 *
 * Investigate's life card is a line of facts, a chart of size over time with a
 * marker on each born, renamed, moved and rewrite, and a row of ticks for each
 * author. All of it is arithmetic on the entries, so it is done here, apart
 * from the component, and tested without a browser.
 *
 * The four targets share one list of entries and differ in what "size" is: a
 * file's lines, a folder's lines, a branch's commits. A line has no size.
 */

import { portraitBackground, seedOf } from '$lib/graph/portrait';
import type { BranchFacts, LifeEntry, LifeKind, LifeTarget, LifeToday } from '$lib/types';

/** How many of a folder's edits are called its biggest. */
export const BIGGEST_EDITS = 20;

/** How many authors get a row of their own. The rest are counted. */
export const AUTHOR_ROWS = 6;

/** The kinds that are drawn as a marker on the chart. */
const MARKED: readonly LifeKind[] = ['born', 'renamed', 'moved', 'rewrite', 'deleted', 'merged'];

/** Each kind's colour, as a token. Meaning, never decoration. */
export const KIND_INK: Record<LifeKind, string> = {
	born: 'var(--ok)',
	renamed: 'var(--lane-5)',
	moved: 'var(--lane-4)',
	rewrite: 'var(--warn)',
	edit: 'var(--muted)',
	deleted: 'var(--danger)',
	added: 'var(--ok)',
	removed: 'var(--danger)',
	commit: 'var(--muted)',
	merge: 'var(--lane-4)',
	merged: 'var(--ok)',
	rebase: 'var(--warn)',
	reset: 'var(--warn)'
};

/**
 * Whether an entry is a commit of the thing itself.
 *
 * A branch's life also holds what is not its own: where it landed, what its
 * reflog says was done to it, and the commit it forked from, which is the
 * other branch's and carries a note saying so.
 */
export function isOwn(entry: LifeEntry): boolean {
	if (entry.kind === 'merged' || entry.kind === 'rebase' || entry.kind === 'reset') return false;
	return !(entry.kind === 'born' && entry.note?.startsWith('from '));
}

/** Who an entry is by: the agent its trailer names, or its author. */
export function byOf(entry: LifeEntry): string {
	return entry.agent ?? entry.authorName;
}

/** The colour a person or an agent is drawn in, here as everywhere. */
export function inkOf(entry: { agent: string | null | boolean; authorEmail: string; authorName: string }): string {
	return entry.agent ? 'var(--agent)' : portraitBackground(seedOf(entry.authorEmail, entry.authorName));
}

/**
 * The size after each entry, aligned with `entries` (newest first).
 *
 * - A **file**: its lines, as counted from the object. A commit whose size
 *   could not be read (a binary version) keeps the size before it.
 * - A **folder**: its lines, summed from the oldest entry up. Only exact once
 *   the whole life is read; before that it is the growth since the oldest
 *   entry on screen.
 * - A **branch**: its commits so far.
 * - A **line**: nothing.
 */
export function sizesOf(kind: LifeTarget['kind'], entries: readonly LifeEntry[]): number[] {
	const out = new Array<number>(entries.length).fill(0);
	if (kind === 'line') return [];
	let running = 0;
	for (let i = entries.length - 1; i >= 0; i--) {
		const entry = entries[i];
		if (kind === 'file') running = entry.size ?? running;
		else if (kind === 'folder') running = Math.max(0, running + entry.added - entry.removed);
		else if (isOwn(entry)) running += 1;
		out[i] = running;
	}
	return out;
}

/** A folder's files after each entry, summed the same way. */
export function filesOf(entries: readonly LifeEntry[]): number[] {
	const out = new Array<number>(entries.length).fill(0);
	let running = 0;
	for (let i = entries.length - 1; i >= 0; i--) {
		running = Math.max(0, running + entries[i].filesAdded - entries[i].filesRemoved);
		out[i] = running;
	}
	return out;
}

/**
 * Which of a folder's entries are listed: the ones that added, removed or
 * renamed a file, where it was born, and the biggest of its edits. A folder
 * thousands of commits have touched is not read as thousands of rows.
 */
export function listed(kind: LifeTarget['kind'], entries: readonly LifeEntry[]): LifeEntry[] {
	if (kind !== 'folder') return [...entries];
	const edits = entries
		.map((entry, index) => ({ index, churn: entry.added + entry.removed, edit: entry.kind === 'edit' }))
		.filter((row) => row.edit && row.churn > 0)
		.sort((a, b) => b.churn - a.churn || a.index - b.index)
		.slice(0, BIGGEST_EDITS);
	const big = new Set(edits.map((row) => row.index));
	return entries.filter((entry, index) => entry.kind !== 'edit' || big.has(index));
}

/** The line of facts above the chart. */
export interface Facts {
	/** Where it was born. Null until the oldest entry has been read. */
	born: LifeEntry | null;
	commits: number;
	authors: number;
	/** Times a file was renamed or moved. */
	renames: number;
	/** Its size now, and the most it has been. Null where there is no size. */
	now: number | null;
	peak: number | null;
	/** What the size counts. */
	unit: 'lines' | 'commits' | null;
	/** A folder: its files now. */
	files: number | null;
	/** A branch: against which branch, and how far ahead and behind. */
	against: string | null;
	ahead: number | null;
	behind: number | null;
}

export function factsOf(
	kind: LifeTarget['kind'],
	entries: readonly LifeEntry[],
	done: boolean,
	today: LifeToday | null,
	branch: BranchFacts | null
): Facts {
	const sizes = sizesOf(kind, entries);
	const commits = entries.filter(isOwn);
	const born = done ? ([...entries].reverse().find((entry) => entry.kind === 'born') ?? null) : null;

	return {
		born,
		commits: commits.length,
		authors: new Set(commits.map(byOf)).size,
		renames: entries.filter((entry) => entry.kind === 'renamed' || entry.kind === 'moved').length,
		now: kind === 'line' ? null : kind === 'file' && today ? today.lines : (sizes[0] ?? null),
		peak: sizes.length === 0 ? null : Math.max(...sizes),
		unit: kind === 'line' ? null : kind === 'branch' ? 'commits' : 'lines',
		files: kind === 'folder' ? (filesOf(entries)[0] ?? 0) : null,
		against: branch?.against ?? null,
		ahead: branch?.against ? branch.ahead : null,
		behind: branch?.against ? branch.behind : null
	};
}

export interface Mark {
	/** From 0 to 1 across the chart. */
	x: number;
	kind: LifeKind;
	label: string;
	ink: string;
}

export interface AuthorRow {
	name: string;
	agent: boolean;
	ink: string;
	/** One per commit, from 0 to 1. */
	ticks: number[];
}

export interface Layout {
	/** The size line, in a box `width` by `height`. Empty where there is no size. */
	line: string;
	/** The same line closed along the bottom, to fill under. */
	area: string;
	marks: Mark[];
	rows: AuthorRow[];
	/** Authors past [`AUTHOR_ROWS`], who have no row. */
	others: number;
	/** Labels along the bottom, evenly spaced, oldest first. */
	axis: string[];
	/** For a screen reader: what the chart shows, in a sentence. */
	said: string;
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const DAY = 86_400;

/** A time on the axis, in as few characters as the span allows. */
function axisLabel(time: number, span: number): string {
	const date = new Date(time * 1000);
	if (span > 730 * DAY) return String(date.getUTCFullYear());
	if (span > 60 * DAY) return MONTHS[date.getUTCMonth()];
	return `${date.getUTCDate()} ${MONTHS[date.getUTCMonth()]}`;
}

/** The label a marker carries. A rewrite says how much of the file it took. */
function markLabel(entry: LifeEntry): string {
	if (entry.kind === 'rewrite' && entry.share !== null) return `rewrite −${Math.round(entry.share * 100)}%`;
	return entry.kind;
}

/**
 * Where everything on the life card goes.
 *
 * Time runs left to right from the oldest entry to the newest. A life of one
 * commit has no span, and is drawn in the middle.
 */
export function layout(
	kind: LifeTarget['kind'],
	entries: readonly LifeEntry[],
	width = 1000,
	height = 120
): Layout {
	const empty: Layout = { line: '', area: '', marks: [], rows: [], others: 0, axis: [], said: '' };
	if (entries.length === 0) return empty;

	const first = entries[entries.length - 1].time;
	const last = entries[0].time;
	const span = Math.max(0, last - first);
	const x = (time: number) => (span === 0 ? 0.5 : Math.min(1, Math.max(0, (time - first) / span)));

	const sizes = sizesOf(kind, entries);
	const peak = sizes.length === 0 ? 0 : Math.max(...sizes);
	let line = '';
	let area = '';
	if (peak > 0) {
		// A little air above the peak, so the line is never on the frame.
		const y = (size: number) => height - (size / peak) * (height - 12);
		const points: [number, number][] = [];
		for (let i = entries.length - 1; i >= 0; i--) {
			points.push([x(entries[i].time) * width, y(sizes[i])]);
		}
		if (points.length === 1) points.unshift([0, points[0][1]], [width, points[0][1]]);
		const path = points.map(([px, py], i) => `${i === 0 ? 'M' : 'L'}${round(px)} ${round(py)}`).join(' ');
		line = path;
		area = `${path} L${round(points[points.length - 1][0])} ${height} L${round(points[0][0])} ${height} Z`;
	}

	const marks = entries
		.filter((entry) => MARKED.includes(entry.kind))
		.map((entry) => ({ x: x(entry.time), kind: entry.kind, label: markLabel(entry), ink: KIND_INK[entry.kind] }))
		.reverse();

	// Authors in the order they first appear, oldest first.
	const byName = new Map<string, AuthorRow>();
	for (let i = entries.length - 1; i >= 0; i--) {
		const entry = entries[i];
		if (!isOwn(entry)) continue;
		const name = byOf(entry);
		let row = byName.get(name);
		if (!row) {
			row = { name, agent: entry.agent !== null, ink: inkOf(entry), ticks: [] };
			byName.set(name, row);
		}
		row.ticks.push(x(entry.time));
	}
	const all = [...byName.values()];

	const steps = span === 0 ? 1 : 8;
	const axis = Array.from({ length: steps }, (_, i) =>
		axisLabel(steps === 1 ? first : first + (span * i) / (steps - 1), span)
	);

	const unit = kind === 'branch' ? 'commits' : 'lines';
	const said =
		peak > 0
			? `${unit === 'lines' ? 'Lines' : 'Commits'} over time: ${sizes[sizes.length - 1]} at first, ${peak} at most, ${sizes[0]} now`
			: '';

	return {
		line,
		area,
		marks,
		rows: all.slice(0, AUTHOR_ROWS),
		others: Math.max(0, all.length - AUTHOR_ROWS),
		axis,
		said
	};
}

function round(value: number): number {
	return Math.round(value * 10) / 10;
}

/** A day and a month, and the year when it is not this one: `14 Mar`, `14 Mar 2024`. */
export function dayOf(unixSeconds: number, now = Date.now()): string {
	const date = new Date(unixSeconds * 1000);
	const day = `${date.getDate()} ${MONTHS[date.getMonth()]}`;
	return date.getFullYear() === new Date(now).getFullYear() ? day : `${day} ${date.getFullYear()}`;
}

/** A target as it is written in the header and the address: a path, a branch, `path:line`. */
export function targetLabel(target: LifeTarget): string {
	switch (target.kind) {
		case 'branch':
			return target.name;
		case 'line':
			return `${target.path}:${target.line}`;
		case 'folder':
			return target.path === '' ? '/' : target.path;
		default:
			return target.path;
	}
}
