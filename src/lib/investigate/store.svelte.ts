// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Investigate screen's state (FEAT-132).
 *
 * One target at a time — a file, a folder, a branch or a line — and its life:
 * the entries read so far, the one selected, and what today's blame says of
 * the file.
 *
 * # Streamed
 *
 * A life is read a page at a time. The first page is on screen before the
 * second is asked for, and the rest fills in behind it; a long history never
 * holds the screen. At `LIFE_MAX` entries the reading stops and the screen
 * offers *Load more*.
 *
 * # Kept
 *
 * A life is kept by its target and by where HEAD is, a few of them. Going back
 * to a file looked at a minute ago costs nothing, and a commit made since
 * reads it again, because the key has changed.
 *
 * # Never out of turn
 *
 * Every reading carries a ticket, so a page for the target before does not
 * land in the life of the target after.
 */

import * as api from '../api';
import type { BranchFacts, Hunk, LifeEntry, LifeTarget, LifeToday } from '../types';
import { listed } from './chart';

/** How many entries are asked for at once. */
export const PAGE = 200;
/** How many are read before the screen offers to read more. Mirrors `investigate::LIFE_MAX`. */
export const LIFE_MAX = 5000;
/** How many lives are kept. */
const KEPT = 8;

/** How far back a life is read. */
export type Range = 'all' | 'year' | 'quarter' | 'month';

export const RANGES: { id: Range; label: string; days: number | null }[] = [
	{ id: 'all', label: 'All time', days: null },
	{ id: 'year', label: 'The last year', days: 365 },
	{ id: 'quarter', label: 'The last 90 days', days: 90 },
	{ id: 'month', label: 'The last 30 days', days: 30 }
];

interface Life {
	entries: LifeEntry[];
	done: boolean;
	today: LifeToday | null;
	branch: BranchFacts | null;
}

let target = $state<LifeTarget | null>(null);
let rev = $state('');
let follow = $state(true);
let range = $state<Range>('all');
/** Where HEAD is, which a kept life is kept by. */
let head = $state('');

let entries = $state.raw<LifeEntry[]>([]);
let done = $state(false);
let reading = $state(false);
/** Reading stopped at the limit, with more to read. */
let more = $state(false);
let error = $state<string | null>(null);

let today = $state.raw<LifeToday | null>(null);
let branch = $state.raw<BranchFacts | null>(null);

let selected = $state.raw<LifeEntry | null>(null);
/** The selected event's largest hunk: `undefined` while it is being read. */
let hunk = $state.raw<Hunk | null | undefined>(undefined);
let comparing = $state(false);
let compared = $state.raw<Hunk[] | null>(null);

const kept = new Map<string, Life>();
let ticket = 0;
let hunkTicket = 0;
/** Where the next page starts. */
let nextSkip = 0;

function keyOf(): string {
	return JSON.stringify([target, rev, follow, range, head]);
}

function since(): number | null {
	const days = RANGES.find((option) => option.id === range)?.days ?? null;
	return days === null ? null : Math.floor(Date.now() / 1000) - days * 86_400;
}

/** The path today's blame is asked of: the file itself, or a line's file. */
function fileOf(of: LifeTarget | null): string | null {
	return of && (of.kind === 'file' || of.kind === 'line') ? of.path : null;
}

function keep(): void {
	const key = keyOf();
	kept.delete(key);
	kept.set(key, { entries, done, today, branch });
	while (kept.size > KEPT) kept.delete(kept.keys().next().value as string);
}

/** A branch's pages are each in order; together they are put in order. */
function merged(kind: LifeTarget['kind'], before: LifeEntry[], page: LifeEntry[]): LifeEntry[] {
	const all = [...before, ...page];
	if (kind !== 'branch') return all;
	return all
		.map((entry, index) => ({ entry, index }))
		.sort((a, b) => b.entry.time - a.entry.time || a.index - b.index)
		.map((row) => row.entry);
}

async function readHunk(entry: LifeEntry): Promise<void> {
	const mine = ++hunkTicket;
	hunk = undefined;
	compared = null;
	comparing = false;
	if (!entry.path || entry.kind === 'deleted') {
		hunk = null;
		return;
	}
	try {
		const next = await api.investigateHunk(entry.commit, entry.path, entry.oldPath);
		if (mine === hunkTicket) hunk = next;
	} catch {
		if (mine === hunkTicket) hunk = null;
	}
}

async function read(from: number): Promise<void> {
	const of = target;
	if (!of) return;
	const mine = ++ticket;
	reading = true;
	error = null;
	more = false;
	let skip = from;
	const limit = from + LIFE_MAX;

	try {
		for (;;) {
			const page = await api.investigateLife({
				target: of,
				rev,
				follow,
				since: since(),
				skip,
				limit: PAGE
			});
			if (mine !== ticket) return;

			const first = entries.length === 0;
			entries = merged(of.kind, entries, page.entries);
			done = page.done;
			skip += PAGE;
			nextSkip = skip;

			if (first) {
				// The first page is on screen: choose something to show beside
				// it, and ask what the file is today, once.
				const shown = listed(of.kind, entries);
				if (shown.length > 0 && !selected) investigate.select(shown[0]);
				const path = fileOf(of);
				if (path && !today) {
					api.investigateToday(path, rev).then(
						(next) => {
							if (mine !== ticket) return;
							today = next;
							// Kept with the life, whichever of the two arrives last.
							if (!reading && !error) keep();
						},
						() => {}
					);
				}
				if (of.kind === 'branch' && !branch) {
					api.investigateBranch(of.name).then(
						(next) => {
							if (mine !== ticket) return;
							branch = next;
							if (!reading && !error) keep();
						},
						() => {}
					);
				}
			}

			if (page.done) break;
			if (skip >= limit) {
				more = true;
				break;
			}
		}
	} catch (failure) {
		if (mine !== ticket) return;
		error = String(failure);
	} finally {
		if (mine === ticket) {
			reading = false;
			if (!error) keep();
		}
	}
}

function begin(): void {
	ticket++;
	hunkTicket++;
	nextSkip = 0;
	entries = [];
	done = false;
	more = false;
	error = null;
	today = null;
	branch = null;
	selected = null;
	hunk = undefined;
	comparing = false;
	compared = null;

	if (!target) {
		reading = false;
		return;
	}
	const held = kept.get(keyOf());
	if (held) {
		entries = held.entries;
		done = held.done;
		today = held.today;
		branch = held.branch;
		reading = false;
		const shown = listed(target.kind, entries);
		if (shown.length > 0) investigate.select(shown[0]);
		return;
	}
	void read(0);
}

export const investigate = {
	get target(): LifeTarget | null {
		return target;
	},
	get rev(): string {
		return rev;
	},
	get follow(): boolean {
		return follow;
	},
	get range(): Range {
		return range;
	},
	get entries(): LifeEntry[] {
		return entries;
	},
	/** The entries the list shows: all of them, or a folder's structural ones and biggest edits. */
	get listed(): LifeEntry[] {
		return target ? listed(target.kind, entries) : [];
	},
	get done(): boolean {
		return done;
	},
	get reading(): boolean {
		return reading;
	},
	get more(): boolean {
		return more;
	},
	get error(): string | null {
		return error;
	},
	get today(): LifeToday | null {
		return today;
	},
	get branch(): BranchFacts | null {
		return branch;
	},
	get selected(): LifeEntry | null {
		return selected;
	},
	get hunk(): Hunk | null | undefined {
		return hunk;
	},
	get comparing(): boolean {
		return comparing;
	},
	get compared(): Hunk[] | null {
		return compared;
	},

	/** Where HEAD is. A life read before a commit is not the life after it. */
	setHead(id: string): void {
		if (id === head) return;
		head = id;
		if (target) begin();
	},

	/** Read the life of `next`, back from `at` (HEAD when not given). */
	show(next: LifeTarget, at = ''): void {
		if (JSON.stringify(next) === JSON.stringify(target) && at === rev && (entries.length > 0 || reading)) {
			return;
		}
		target = next;
		rev = at;
		begin();
	},

	setFollow(on: boolean): void {
		if (on === follow) return;
		follow = on;
		if (target) begin();
	},

	setRange(next: Range): void {
		if (next === range) return;
		range = next;
		if (target) begin();
	},

	/** Go on reading past the limit. */
	loadMore(): void {
		if (!more || reading) return;
		void read(nextSkip);
	},

	select(entry: LifeEntry): void {
		if (entry === selected) return;
		selected = entry;
		void readHunk(entry);
	},

	/** How many of the selected commit's lines today's blame still gives it. */
	get alive(): number | null {
		if (!today || !selected) return null;
		return today.alive[selected.commit] ?? 0;
	},

	/**
	 * The file as the selected commit left it against the file today, in
	 * place of the commit's own largest hunk. Pressed again, back to the hunk.
	 */
	async toggleCompare(): Promise<void> {
		const entry = selected;
		const path = fileOf(target);
		if (!entry?.path || !path) return;
		if (comparing) {
			comparing = false;
			return;
		}
		comparing = true;
		if (compared) return;
		const mine = hunkTicket;
		try {
			const next = await api.investigateCompare(entry.commit, entry.path, rev, path);
			if (mine === hunkTicket) compared = next;
		} catch {
			if (mine === hunkTicket) compared = [];
		}
	},

	clearState(): void {
		target = null;
		rev = '';
		follow = true;
		range = 'all';
		head = '';
		kept.clear();
		begin();
	}
};
