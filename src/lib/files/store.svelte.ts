// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Files screen's state (FEAT-131).
 *
 * Files shows the repository at a revision: a tree, the files opened from it
 * as tabs, and one of them read. Everything is read from git's objects — see
 * `crates/spagitty-core/src/files.rs` — so nothing here has a save, a dirty
 * flag or a draft, and there is nothing to disable.
 *
 * # The order things arrive in
 *
 * A file's **text** is read and shown at once. Its **colour** is asked for a
 * screenful at a time, when rows are about to be drawn, and fills in after.
 * Its **blame** is asked for after the first paint and fills the gutter when
 * it comes. A 20,000-line file therefore costs its text to open, and what is
 * in view to colour; the item asks for exactly that order.
 *
 * # Never out of turn
 *
 * Every read carries a ticket. A person clicking down a tree asks for five
 * files in a second, and an answer for the second must not land on the fifth.
 *
 * # What is remembered
 *
 * Per repository, in `localStorage`: the revision, the tabs, the open folders
 * and the view switches. A convenience, so it may be lost.
 */

import * as api from '../api';
import { detectLanguage, tokenizeLines, type Token } from '../diff/highlight';
import type {
	BlameCommit,
	CompactBlame,
	FilesEntry,
	FilesFound,
	FilesHit,
	FilesRead,
	FilesRevision,
	FilesSearched,
	SearchOptions
} from '../types';
import { linesOf, tokensOf } from './colour';
import { blockFrom, find, PLAIN, patternOf, type Match } from './keys';

/** One open file: a path, and the revision it was opened at. */
export interface Tab {
	path: string;
	/** A branch, a tag or a commit, as it was asked for. */
	rev: string;
	/** The line the cursor was on, 0-based, kept while the tab is. */
	line: number;
}

/** A folder as the tree holds it: its entries, or why there are none yet. */
export type Folder = FilesEntry[] | 'loading' | { error: string };

/** How many lines of colour are asked for at once. */
export const COLOUR_PAGE = 400;

const KEY = 'spagitty.files';
const TABS_KEPT = 30;

interface Remembered {
	rev: string;
	tabs: { path: string; rev: string }[];
	active: number;
	open: string[];
	blame: boolean;
	wrap: boolean;
}

let repo = $state<string | null>(null);
let rev = $state('');
let resolved = $state<FilesRevision | null>(null);
let revisionError = $state<string | null>(null);

let tabs = $state<Tab[]>([]);
let active = $state(-1);

let open = $state<string[]>([]);
let folders = $state<Record<string, Folder>>({});

let blameOn = $state(true);
let wrap = $state(false);

let file = $state.raw<FilesRead | null>(null);
let lines = $state.raw<string[]>([]);
let reading = $state(false);
let fileError = $state<string | null>(null);

/** Colour by line, sparse: a line not asked for yet is absent. */
let colours: (number[] | undefined)[] = [];
/** The pages of colour asked for, so none is asked for twice. */
let pages = new Set<number>();
/** The other highlighter's answer, for a language with no grammar. */
let fallback: Token[][] | null = null;
/** Bumped when colour arrives, which is what redraws the rows that got it. */
let painted = $state(0);

let blame = $state.raw<CompactBlame | null>(null);
let blaming = $state(false);
let blameError = $state<string | null>(null);

let cursor = $state({ line: 0, col: 0 });
/** The other end of a shift-click range, or null for no range. */
let anchor = $state<number | null>(null);
let peek = $state<number | null>(null);
/** What the peeked line was before: `undefined` while it is being read. */
let was = $state<string | null | undefined>(undefined);
/** The commit whose lines are lit, as an index into the blame's commits. */
let lit = $state<number | null>(null);

let searching = $state(false);
let backward = $state(false);
let query = $state('');
let match = $state<Match | null>(null);
/** The last search found nothing. */
let nowhere = $state(false);

let finding = $state('');
let found = $state<FilesFound | null>(null);

/** How both searches read what was typed: regex, whole word, case (FEAT-151). */
let options = $state<SearchOptions>({ ...PLAIN });
/** What the tree's field looks for: file names, or text in every file. */
let lookFor = $state<'names' | 'text'>('names');
let seeking = $state('');
let searched = $state.raw<FilesSearched | null>(null);
let searchedError = $state<string | null>(null);
let seekingNow = $state(false);
let seekTicket = 0;

let fileTicket = 0;
let blameTicket = 0;
let peekTicket = 0;
let findTicket = 0;
let revTicket = 0;

function remembered(path: string): Remembered | null {
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, Remembered>;
		return all[path] ?? null;
	} catch {
		return null;
	}
}

function remember(): void {
	if (!repo) return;
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, Remembered>;
		all[repo] = {
			rev,
			tabs: tabs.slice(-TABS_KEPT).map((tab) => ({ path: tab.path, rev: tab.rev })),
			active,
			open,
			blame: blameOn,
			wrap
		};
		localStorage.setItem(KEY, JSON.stringify(all));
	} catch {
		// Storage is unavailable: the tabs are only for this visit.
	}
}

/** Run `work` once the frame that shows the text has been drawn. */
function afterPaint(work: () => void): void {
	if (typeof requestAnimationFrame === 'function') requestAnimationFrame(() => setTimeout(work, 0));
	else setTimeout(work, 0);
}

function clearFile(): void {
	file = null;
	lines = [];
	colours = [];
	pages = new Set();
	fallback = null;
	blame = null;
	blaming = false;
	blameError = null;
	peek = null;
	was = undefined;
	lit = null;
	anchor = null;
	match = null;
	nowhere = false;
	fileError = null;
}

async function readFolder(dir: string): Promise<void> {
	if (!resolved) return;
	const at = resolved.id;
	const held = folders[dir];
	if (Array.isArray(held) || held === 'loading') return;
	folders[dir] = 'loading';
	try {
		const entries = await api.filesTree(at, dir);
		if (resolved?.id !== at) return;
		folders[dir] = entries;
	} catch (error) {
		if (resolved?.id !== at) return;
		folders[dir] = { error: String(error) };
		// A folder this revision does not have is not an open folder.
		open = open.filter((path) => path !== dir && !path.startsWith(`${dir}/`));
	}
}

/** Resolve the screen's revision and read the tree again at it. */
async function resolveNow(): Promise<boolean> {
	const ticket = ++revTicket;
	try {
		const next = await api.filesResolve(rev);
		if (ticket !== revTicket) return false;
		revisionError = null;
		if (next.id !== resolved?.id) {
			resolved = next;
			folders = {};
			void readFolder('');
			for (const dir of open) void readFolder(dir);
			// A search of the files is of this revision's files.
			if (lookFor === 'text' && seeking !== '') void files.searchFiles(seeking);
		}
		return true;
	} catch (error) {
		if (ticket !== revTicket) return false;
		revisionError = String(error);
		return false;
	}
}

async function readActive(): Promise<void> {
	const tab = tabs[active];
	const ticket = ++fileTicket;
	blameTicket++;
	peekTicket++;
	clearFile();
	if (!tab) {
		reading = false;
		return;
	}
	reading = true;
	try {
		const read = await api.filesRead(tab.rev, tab.path);
		if (ticket !== fileTicket) return;
		file = read;
		lines = read.content.kind === 'text' ? linesOf(read.content.text) : [];
		const last = Math.max(0, lines.length - 1);
		cursor = { line: Math.min(Math.max(tab.line, 0), last), col: 0 };
		reading = false;

		// Colour and blame come after the text is on screen.
		if (read.content.kind !== 'text') return;
		afterPaint(() => {
			if (ticket !== fileTicket) return;
			if (!read.grammar && read.colour) colourWithoutGrammar(ticket);
			if (blameOn) void readBlame();
		});
	} catch (error) {
		if (ticket !== fileTicket) return;
		reading = false;
		fileError = String(error);
	}
}

/** A language tree-sitter has no grammar for: the highlighter Diff uses. */
function colourWithoutGrammar(ticket: number): void {
	if (!file) return;
	const language = detectLanguage(file.path);
	if (language === 'plain') return;
	const tokens = tokenizeLines(lines, language);
	if (ticket !== fileTicket) return;
	fallback = tokens;
	painted += 1;
}

async function readBlame(): Promise<void> {
	const read = file;
	if (!read || read.content.kind !== 'text' || blame || blaming) return;
	const ticket = ++blameTicket;
	blaming = true;
	blameError = null;
	try {
		// By the commit the file was read from, so the two are of one revision
		// even when the branch has moved since.
		const next = await api.filesBlame(read.revision, read.path);
		if (ticket !== blameTicket) return;
		blame = next;
		if (peek !== null) void readWas(peek);
	} catch (error) {
		if (ticket !== blameTicket) return;
		blameError = String(error);
	} finally {
		if (ticket === blameTicket) blaming = false;
	}
}

async function readWas(line: number): Promise<void> {
	const ticket = ++peekTicket;
	was = undefined;
	const commit = files.commitOf(line);
	if (!blame || !commit) return;
	try {
		const text = await api.filesBefore(
			commit.id,
			commit.path,
			blame.origin[line],
			commit.previous,
			commit.previousPath
		);
		if (ticket === peekTicket) was = text;
	} catch {
		// The peek still says who and why; what the line was is the part
		// that could not be read.
		if (ticket === peekTicket) was = null;
	}
}

export const files = {
	get repo(): string | null {
		return repo;
	},
	/** The revision as it was asked for: a branch, a tag or a commit. */
	get rev(): string {
		return rev;
	},
	get resolved(): FilesRevision | null {
		return resolved;
	},
	get revisionError(): string | null {
		return revisionError;
	},
	get tabs(): Tab[] {
		return tabs;
	},
	get active(): number {
		return active;
	},
	get tab(): Tab | null {
		return tabs[active] ?? null;
	},
	get open(): string[] {
		return open;
	},
	folder(dir: string): Folder | undefined {
		return folders[dir];
	},
	get blameOn(): boolean {
		return blameOn;
	},
	get wrap(): boolean {
		return wrap;
	},
	get file(): FilesRead | null {
		return file;
	},
	get lines(): string[] {
		return lines;
	},
	get reading(): boolean {
		return reading;
	},
	get fileError(): string | null {
		return fileError;
	},
	get blame(): CompactBlame | null {
		return blame;
	},
	get blaming(): boolean {
		return blaming;
	},
	get blameError(): string | null {
		return blameError;
	},
	get cursor(): { line: number; col: number } {
		return cursor;
	},
	/** The lines selected, `[first, last]`, 0-based and inclusive. */
	get selection(): [number, number] {
		const other = anchor ?? cursor.line;
		return [Math.min(other, cursor.line), Math.max(other, cursor.line)];
	},
	get peek(): number | null {
		return peek;
	},
	get was(): string | null | undefined {
		return was;
	},
	get lit(): number | null {
		return lit;
	},
	get searching(): boolean {
		return searching;
	},
	get backward(): boolean {
		return backward;
	},
	get query(): string {
		return query;
	},
	get match(): Match | null {
		return match;
	},
	get nowhere(): boolean {
		return nowhere;
	},
	get finding(): string {
		return finding;
	},
	get found(): FilesFound | null {
		return found;
	},
	get searchOptions(): SearchOptions {
		return options;
	},
	/** Why what is typed in the search is not a regular expression, or null. */
	get searchError(): string | null {
		if (query === '') return null;
		const pattern = patternOf(query, options);
		return typeof pattern === 'string' ? pattern : null;
	},
	get lookFor(): 'names' | 'text' {
		return lookFor;
	},
	get seeking(): string {
		return seeking;
	},
	get searched(): FilesSearched | null {
		return searched;
	},
	get searchedError(): string | null {
		return searchedError;
	},
	get seekingNow(): boolean {
		return seekingNow;
	},

	/** The language as the status line names it. */
	get language(): string {
		return file ? detectLanguage(file.path) : 'plain';
	},
	/** Whether the colour on screen is tree-sitter's. */
	get treeSitter(): boolean {
		return !!file?.grammar;
	},

	/** A line's coloured runs. Plain until its colour has arrived. */
	tokens(line: number): Token[] {
		void painted;
		const text = lines[line] ?? '';
		if (file?.grammar) return tokensOf(text, colours[line]);
		return fallback?.[line] ?? tokensOf(text, undefined);
	},

	/** The commit that wrote a line, once the blame is in. */
	commitOf(line: number): BlameCommit | null {
		if (!blame) return null;
		const index = blame.lines[line];
		return index === undefined ? null : (blame.commits[index] ?? null);
	},

	/**
	 * Open this repository's Files: what was remembered, at `HEAD`'s branch
	 * otherwise. Called when the screen is shown and when the repository
	 * changes; the same repository again only looks for a moved revision.
	 */
	async enter(path: string, branch: string | null): Promise<void> {
		if (repo === path) {
			await this.refresh();
			return;
		}
		repo = path;
		fileTicket++;
		clearFile();
		resolved = null;
		folders = {};
		found = null;
		finding = '';

		const kept = remembered(path);
		rev = kept?.rev || branch || 'HEAD';
		tabs = (kept?.tabs ?? []).map((tab) => ({ ...tab, line: 0 }));
		active = tabs.length === 0 ? -1 : Math.min(Math.max(kept?.active ?? 0, 0), tabs.length - 1);
		open = kept?.open ?? [];
		blameOn = kept?.blame ?? true;
		wrap = kept?.wrap ?? false;

		const ok = await resolveNow();
		// A branch that has gone since the last visit: back to where HEAD is.
		if (!ok && rev !== 'HEAD' && repo === path) {
			rev = branch || 'HEAD';
			await resolveNow();
		}
		if (repo === path) await readActive();
	},

	/**
	 * Look again at what the revision means. A branch moves — a commit, a
	 * pull — and the tree and the file read at its old tip are then of a
	 * commit the branch is no longer at.
	 */
	async refresh(): Promise<void> {
		const before = resolved?.id;
		await resolveNow();
		if (resolved?.id !== before && file && tabs[active]?.rev === rev) await readActive();
	},

	/** Read everything at another revision: the tree, and the open file. */
	async setRevision(next: string): Promise<void> {
		const name = next.trim();
		if (name === '' || name === rev) return;
		const before = rev;
		rev = name;
		if (!(await resolveNow())) {
			// Not a revision: say so, and stay where we were.
			const said = revisionError;
			rev = before;
			await resolveNow();
			revisionError = said;
			return;
		}
		const tab = tabs[active];
		if (tab) {
			tab.rev = name;
			await readActive();
		}
		remember();
	},

	/**
	 * Open a file, in the tab it already has or a new one. With a revision,
	 * the screen goes to that revision first: a file opened from another
	 * screen opens at the revision that screen was showing.
	 */
	async openFile(path: string, at?: { rev?: string; line?: number }): Promise<void> {
		const wanted = at?.rev?.trim() || rev;
		if (wanted !== rev) {
			rev = wanted;
			await resolveNow();
		}
		const line = Math.max(0, (at?.line ?? 1) - 1);
		// The tree shows where the file is, however it was opened.
		const cut = path.lastIndexOf('/');
		if (cut !== -1) this.reveal(path.slice(0, cut));
		const index = tabs.findIndex((tab) => tab.path === path && tab.rev === wanted);
		const here = tabs[active];
		if (here) here.line = cursor.line;
		if (index === -1) {
			tabs.push({ path, rev: wanted, line });
			active = tabs.length - 1;
		} else {
			// Already the file on screen: go to the line asked for, if one was.
			if (index === active && file?.path === path && file.content.kind !== 'missing') {
				if (at?.line !== undefined) this.moveTo(line);
				return;
			}
			active = index;
			if (at?.line !== undefined) tabs[index].line = line;
		}
		remember();
		await readActive();
	},

	/** Show a tab. One opened at another revision takes the screen there. */
	async activate(index: number): Promise<void> {
		const tab = tabs[index];
		if (!tab || index === active) return;
		const here = tabs[active];
		if (here) here.line = cursor.line;
		active = index;
		if (tab.rev !== rev) {
			rev = tab.rev;
			await resolveNow();
		}
		remember();
		await readActive();
	},

	async close(index: number): Promise<void> {
		if (!tabs[index]) return;
		const wasActive = index === active;
		tabs.splice(index, 1);
		if (index < active) active -= 1;
		else if (wasActive) active = Math.min(index, tabs.length - 1);
		remember();
		if (wasActive) {
			const next = tabs[active];
			if (next && next.rev !== rev) {
				rev = next.rev;
				await resolveNow();
			}
			await readActive();
		}
	},

	/** Open or close a folder. Its entries are read the first time. */
	toggleFolder(dir: string): void {
		if (open.includes(dir)) {
			open = open.filter((path) => path !== dir);
		} else {
			open = [...open, dir];
			void readFolder(dir);
		}
		remember();
	},

	/** Open every folder down to `dir`, so the tree shows it. */
	reveal(dir: string): void {
		const parts = dir.split('/').filter(Boolean);
		const wanted = parts.map((_, index) => parts.slice(0, index + 1).join('/'));
		const missing = wanted.filter((path) => !open.includes(path));
		if (missing.length === 0) return;
		open = [...open, ...missing];
		for (const path of missing) void readFolder(path);
		remember();
	},

	/** Ask for the colour of the rows about to be drawn, a page at a time. */
	wantColour(start: number, end: number): void {
		const read = file;
		if (!read?.grammar || !read.colour || lines.length === 0) return;
		const grammar = read.grammar;
		const first = Math.floor(Math.max(0, start) / COLOUR_PAGE);
		const last = Math.floor(Math.max(0, Math.min(end, lines.length) - 1) / COLOUR_PAGE);
		for (let page = first; page <= last; page++) {
			if (pages.has(page)) continue;
			pages.add(page);
			const from = page * COLOUR_PAGE;
			const ticket = fileTicket;
			const asked = pages;
			api.filesSpans(read.blob, grammar, from, from + COLOUR_PAGE).then(
				(spans) => {
					if (ticket !== fileTicket) return;
					for (let i = 0; i < spans.length; i++) colours[from + i] = spans[i];
					painted += 1;
				},
				() => {
					// Left plain. Asked for again if these rows are drawn again.
					if (ticket === fileTicket) asked.delete(page);
				}
			);
		}
	},

	toggleBlame(): void {
		blameOn = !blameOn;
		if (blameOn) void readBlame();
		else lit = null;
		remember();
	},

	toggleWrap(): void {
		wrap = !wrap;
		remember();
	},

	/** Put the cursor on a line. With `extend`, select from where it was. */
	moveTo(line: number, extend = false): void {
		if (lines.length === 0) return;
		const next = Math.min(Math.max(line, 0), lines.length - 1);
		anchor = extend ? (anchor ?? cursor.line) : null;
		cursor = { line: next, col: 0 };
		if (peek !== null && peek !== next) this.closePeek();
	},

	moveBy(delta: number): void {
		this.moveTo(cursor.line + delta);
	},

	/** The next blame block's first line, or the one before. */
	moveBlock(by: number): void {
		if (!blame) return;
		this.moveTo(blockFrom(blame.lines, cursor.line, by));
	},

	/** Light every line a commit wrote. The same block again puts them out. */
	light(line: number): void {
		const index = blame?.lines[line];
		if (index === undefined) return;
		lit = lit === index ? null : index;
	},

	/** Open the peek under a line. It needs the blame, so it asks for it. */
	openPeek(line: number): void {
		if (lines.length === 0) return;
		this.moveTo(line);
		peek = cursor.line;
		was = undefined;
		if (blame) void readWas(cursor.line);
		else void readBlame();
	},

	closePeek(): void {
		peek = null;
		was = undefined;
		peekTicket++;
	},

	/** Blame again at the commit before the one that wrote this line. */
	async blameBefore(line: number): Promise<void> {
		const commit = this.commitOf(line);
		if (!commit?.previous) return;
		await this.openFile(commit.previousPath ?? commit.path, { rev: commit.previous });
	},

	startSearch(reverse: boolean): void {
		searching = true;
		backward = reverse;
		nowhere = false;
	},

	setQuery(text: string): void {
		query = text;
		nowhere = false;
	},

	/** Leave the search field. The word stays, for `n` and `N`. */
	endSearch(): void {
		searching = false;
	},

	/**
	 * Go to the next place the word is found. `reverse` turns the search's
	 * own direction round, as `N` does.
	 */
	searchNext(reverse = false): Match | null {
		if (query === '') return null;
		const next = find(lines, query, cursor, backward !== reverse, options);
		nowhere = next === null;
		if (!next) return null;
		anchor = null;
		cursor = { line: next.line, col: next.start };
		match = next;
		if (peek !== null && peek !== next.line) this.closePeek();
		return next;
	},

	/** Put out whatever is lit, selected or found, nearest first. */
	dismiss(): void {
		if (peek !== null) this.closePeek();
		else if (match) match = null;
		else if (anchor !== null) anchor = null;
		else lit = null;
	},

	/** `path:line@sha`, or `path:first-last@sha` for a range: what `y` copies. */
	get reference(): string | null {
		if (!file || lines.length === 0) return null;
		const [first, last] = this.selection;
		const where = first === last ? `${first + 1}` : `${first + 1}-${last + 1}`;
		return `${file.path}:${where}@${file.revision.slice(0, 7)}`;
	},

	/**
	 * Turn regex, whole word or match case on or off, for both searches. A
	 * search of the files already shown is run again with it.
	 */
	toggleOption(name: keyof SearchOptions): void {
		options = { ...options, [name]: !options[name] };
		nowhere = false;
		if (lookFor === 'text' && seeking !== '') void this.searchFiles(seeking);
	},

	/** What the tree's field looks for. The other one's answer is put away. */
	setLookFor(next: 'names' | 'text'): void {
		if (next === lookFor) return;
		lookFor = next;
		if (next === 'text') {
			void this.findFiles('');
			if (seeking !== '') void this.searchFiles(seeking);
		} else {
			seekTicket++;
			searched = null;
			searchedError = null;
			seekingNow = false;
		}
	},

	/** Every line of every file at the revision that `text` matches (FEAT-151). */
	async searchFiles(text: string): Promise<void> {
		seeking = text;
		const ticket = ++seekTicket;
		searchedError = null;
		if (text === '' || !resolved) {
			searched = null;
			seekingNow = false;
			return;
		}
		const pattern = patternOf(text, options);
		if (typeof pattern === 'string') {
			searched = null;
			searchedError = pattern;
			seekingNow = false;
			return;
		}
		seekingNow = true;
		try {
			const next = await api.filesSearch(resolved.id, text, options);
			if (ticket === seekTicket) searched = next;
		} catch (error) {
			if (ticket === seekTicket) {
				searched = null;
				searchedError = String(error);
			}
		} finally {
			if (ticket === seekTicket) seekingNow = false;
		}
	},

	/**
	 * Open a file a search found, at its line, with its match marked; the
	 * search becomes the file's own, so `n` and `N` carry on through it.
	 */
	async openHit(hit: FilesHit): Promise<void> {
		await this.openFile(hit.path, { line: hit.line });
		if (file?.path !== hit.path) return;
		query = seeking;
		backward = false;
		nowhere = false;
		match = { line: hit.line - 1, start: hit.column, end: hit.column + hit.length };
		cursor = { line: hit.line - 1, col: hit.column };
	},

	/** Look for files by name. An empty word goes back to the tree. */
	async findFiles(text: string): Promise<void> {
		finding = text;
		const ticket = ++findTicket;
		if (text.trim() === '' || !resolved) {
			found = null;
			return;
		}
		try {
			const next = await api.filesFind(resolved.id, text);
			if (ticket === findTicket) found = next;
		} catch {
			if (ticket === findTicket) found = { paths: [], more: false };
		}
	},

	clearState(): void {
		repo = null;
		rev = '';
		resolved = null;
		revisionError = null;
		tabs = [];
		active = -1;
		open = [];
		folders = {};
		blameOn = true;
		wrap = false;
		reading = false;
		cursor = { line: 0, col: 0 };
		searching = false;
		backward = false;
		query = '';
		finding = '';
		found = null;
		options = { ...PLAIN };
		lookFor = 'names';
		seeking = '';
		searched = null;
		searchedError = null;
		seekingNow = false;
		seekTicket++;
		fileTicket++;
		blameTicket++;
		peekTicket++;
		findTicket++;
		revTicket++;
		clearFile();
	}
};

/**
 * A revision as it is shown: a commit's full id is cut to seven characters,
 * and a branch or a tag is its name.
 */
export function revisionLabel(rev: string): string {
	return /^[0-9a-f]{40}$/.test(rev) ? rev.slice(0, 7) : rev;
}

/**
 * The pull request a commit's summary names, as a host writes it: `(#214)`
 * at the end of a squashed commit, or `Merge pull request #214`. Null when
 * the summary names none.
 */
export function pullOf(summary: string): number | null {
	const found = /\(#(\d+)\)\s*$/.exec(summary) ?? /^Merge pull request #(\d+)\b/.exec(summary);
	return found ? Number(found[1]) : null;
}
