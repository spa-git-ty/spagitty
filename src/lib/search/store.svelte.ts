// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Log search.
 *
 * Results stream in as the walk finds them, so the first match is on screen
 * long before the walk reaches the end of history. Every query carries a token
 * and rows from any other one are dropped — which is what makes it safe to
 * start a new query on every keystroke.
 *
 * Rows live in a plain array rather than in `$state` for the same reason the
 * graph's do: a query matching thirty thousand commits would otherwise mean
 * thirty thousand deeply-proxied objects. `version` is the reactive signal.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from '../api';
import {
	SEARCH_DONE_EVENT,
	SEARCH_ROWS_EVENT,
	type Blame,
	type CommitDetail,
	type SearchDoneEvent,
	type SearchQuery,
	type SearchRow,
	type SearchRowsEvent
} from '../types';

/** Untracked row storage, indexed by position in the result. */
let buffer: SearchRow[] = [];

let version = $state(0);
let count = $state(0);
let running = $state(false);
let complete = $state(false);
let error = $state<string | null>(null);
let ran = $state(false);

let author = $state('');
let message = $state('');
let path = $state('');
let diffContent = $state('');
let since = $state('');
let until = $state('');

/** The query the rows on screen answer, so an empty result can name it. */
let asked = $state<SearchQuery | null>(null);

let selectedId = $state<string | null>(null);
let detail = $state<CommitDetail | null>(null);
let detailError = $state<string | null>(null);

let blame = $state<Blame | null>(null);
let blamePath = $state<string | null>(null);
let blameError = $state<string | null>(null);
let blaming = $state(false);

let token: number | null = null;
let detailSeq = 0;
let blameSeq = 0;
let unlisteners: UnlistenFn[] = [];

/** A date field is `YYYY-MM-DD`; anything else is not a date and is ignored. */
function toSeconds(value: string, endOfDay: boolean): number | null {
	const text = value.trim();
	if (!/^\d{4}-\d{2}-\d{2}$/.test(text)) return null;
	const at = Date.parse(endOfDay ? `${text}T23:59:59Z` : `${text}T00:00:00Z`);
	return Number.isNaN(at) ? null : Math.floor(at / 1000);
}

function trimmed(value: string): string | null {
	const text = value.trim();
	return text === '' ? null : text;
}

/** What the fields currently add up to. */
function current(): SearchQuery {
	return {
		author: trimmed(author),
		message: trimmed(message),
		path: trimmed(path),
		diffContent: trimmed(diffContent),
		since: toSeconds(since, false),
		until: toSeconds(until, true)
	};
}

function isEmpty(query: SearchQuery): boolean {
	return (
		!query.author && !query.message && !query.path && !query.diffContent && query.since == null && query.until == null
	);
}

/**
 * The chips, in the order they were listed in the design. Derived rather than
 * stored, so a chip and its field cannot disagree about what is applied.
 */
function chipsFor(query: SearchQuery): Array<{ key: keyof SearchQuery; label: string }> {
	const out: Array<{ key: keyof SearchQuery; label: string }> = [];
	if (query.author) out.push({ key: 'author', label: `author:${query.author}` });
	if (query.path) out.push({ key: 'path', label: `path:${query.path}` });
	if (query.diffContent) out.push({ key: 'diffContent', label: `diff:${query.diffContent}` });
	if (query.message) out.push({ key: 'message', label: `message:${query.message}` });
	if (query.since != null) out.push({ key: 'since', label: `since:${since.trim()}` });
	if (query.until != null) out.push({ key: 'until', label: `until:${until.trim()}` });
	return out;
}

/**
 * Which filter is narrowest, for an empty result to point at.
 *
 * The same order the core uses: a path is the most specific thing anyone types
 * here, then a message, then an author, then a date. Naming one is a guess, and
 * a better one than "no results".
 */
function narrowest(query: SearchQuery): string | null {
	if (query.path) return `path:${query.path}`;
	if (query.diffContent) return `diff:${query.diffContent}`;
	if (query.message) return `message:${query.message}`;
	if (query.author) return `author:${query.author}`;
	if (query.since != null || query.until != null) return 'the date range';
	return null;
}

export const search = {
	/** For tests and previews: results and an opened commit, without the backend. */
	seed(next: { author?: string; message?: string; path?: string; rows: SearchRow[]; detail?: CommitDetail | null }) {
		author = next.author ?? '';
		message = next.message ?? '';
		path = next.path ?? '';
		buffer = [...next.rows];
		count = next.rows.length;
		version += 1;
		ran = true;
		running = false;
		complete = true;
		error = null;
		detail = next.detail ?? null;
		selectedId = next.detail?.id ?? null;
	},

	get version(): number {
		return version;
	},
	get count(): number {
		return count;
	},
	get running(): boolean {
		return running;
	},
	get complete(): boolean {
		return complete;
	},
	get error(): string | null {
		return error;
	},
	/** True once a query has been run, so an untouched screen is not "no results". */
	get ran(): boolean {
		return ran;
	},
	get selectedId(): string | null {
		return selectedId;
	},
	get detail(): CommitDetail | null {
		return detail;
	},
	get detailError(): string | null {
		return detailError;
	},

	get author(): string {
		return author;
	},
	set author(value: string) {
		author = value;
	},
	get message(): string {
		return message;
	},
	set message(value: string) {
		message = value;
	},
	get path(): string {
		return path;
	},
	set path(value: string) {
		path = value;
	},
	get diffContent(): string {
		return diffContent;
	},
	set diffContent(value: string) {
		diffContent = value;
	},
	get since(): string {
		return since;
	},
	set since(value: string) {
		since = value;
	},
	get until(): string {
		return until;
	},
	set until(value: string) {
		until = value;
	},

	/** What is typed right now, as chips. */
	get chips(): Array<{ key: keyof SearchQuery; label: string }> {
		return chipsFor(current());
	},

	/** True when nothing is being asked, which is not a question. */
	get empty(): boolean {
		return isEmpty(current());
	},

	/** The narrowest filter of the query the rows answer. */
	get narrowestApplied(): string | null {
		return asked ? narrowest(asked) : null;
	},

	get blame(): Blame | null {
		return blame;
	},
	get blamePath(): string | null {
		return blamePath;
	},
	get blameError(): string | null {
		return blameError;
	},
	get blaming(): boolean {
		return blaming;
	},

	row(index: number): SearchRow | undefined {
		return buffer[index];
	},

	/** Every loaded row. For the list, which is not virtualised. */
	rows(): SearchRow[] {
		void version;
		return buffer.slice(0, count);
	},

	/** Subscribe to the walk. Call once; the returned function detaches. */
	async attach(): Promise<() => void> {
		unlisteners.push(
			await listen<SearchRowsEvent>(SEARCH_ROWS_EVENT, (event) => {
				if (event.payload.token !== token) return;
				for (const row of event.payload.rows) {
					buffer[row.index] = row;
					if (row.index + 1 > count) count = row.index + 1;
				}
				version += 1;
			})
		);

		unlisteners.push(
			await listen<SearchDoneEvent>(SEARCH_DONE_EVENT, (event) => {
				if (event.payload.token !== token) return;
				running = false;
				complete = event.payload.complete;
				if (event.payload.error) error = event.payload.error;
			})
		);

		return () => {
			for (const off of unlisteners) off();
			unlisteners = [];
		};
	},

	/** Run what is typed. An empty query is refused rather than run. */
	async run(): Promise<void> {
		const query = current();
		if (isEmpty(query)) return;

		buffer = [];
		count = 0;
		version += 1;
		complete = false;
		error = null;
		running = true;
		ran = true;
		asked = query;
		selectedId = null;
		detailSeq += 1;
		detail = null;
		detailError = null;

		try {
			token = await api.searchStart(query);
		} catch (e) {
			error = String(e);
			running = false;
			token = null;
		}
	},

	/** Remove one filter and re-run, which is what a chip's × does. */
	async removeChip(key: keyof SearchQuery): Promise<void> {
		if (key === 'author') author = '';
		if (key === 'message') message = '';
		if (key === 'path') path = '';
		if (key === 'diffContent') diffContent = '';
		if (key === 'since') since = '';
		if (key === 'until') until = '';
		if (this.empty) {
			this.clearResults();
			return;
		}
		await this.run();
	},

	/**
	 * Open a result. Reading its detail is what "opens the commit" means here:
	 * the message, the people and the files it touched, beside the results
	 * rather than instead of them.
	 */
	async select(id: string | null): Promise<void> {
		selectedId = id;
		const current = ++detailSeq;
		if (id === null) {
			detail = null;
			detailError = null;
			return;
		}
		try {
			const next = await api.commitDetail(id);
			if (current !== detailSeq) return;
			detail = next;
			detailError = null;
		} catch (e) {
			if (current === detailSeq) {
				detailError = String(e);
				detail = null;
			}
		}
	},

	/** Blame a file at a revision. An empty revision means HEAD. */
	async loadBlame(file: string, revision = ''): Promise<void> {
		const wanted = file.trim();
		if (wanted === '') return;

		blaming = true;
		blamePath = wanted;
		const current = ++blameSeq;
		try {
			const next = await api.blame(wanted, revision);
			if (current !== blameSeq) return;
			blame = next;
			blameError = null;
		} catch (e) {
			if (current === blameSeq) {
				blameError = String(e);
				blame = null;
			}
		} finally {
			if (current === blameSeq) blaming = false;
		}
	},

	clearBlame(): void {
		blameSeq += 1;
		blame = null;
		blamePath = null;
		blameError = null;
		blaming = false;
	},

	/** Drop the results but keep what is typed. */
	clearResults(): void {
		token = null;
		buffer = [];
		count = 0;
		version += 1;
		running = false;
		complete = false;
		error = null;
		ran = false;
		asked = null;
		selectedId = null;
		detailSeq += 1;
		detail = null;
		detailError = null;
	},

	/** Stop the running query and forget everything. */
	async stop(): Promise<void> {
		this.clearResults();
		this.clearBlame();
		author = '';
		message = '';
		path = '';
		since = '';
		diffContent = '';
		until = '';
		try {
			await api.searchStop();
		} catch {
			// Nothing was running, which is not a failure.
		}
	}
};
