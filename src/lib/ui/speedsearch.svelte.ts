// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Type to search, in any list (FEAT-158).
 *
 * With no text field focused, typing a letter on a screen opens a small search
 * over its list, the way IntelliJ's speed search does: there is no bar to click
 * first. The list narrows to what matches, Backspace takes a letter back,
 * Escape puts the whole list back, and Enter opens the first match.
 *
 * A list says it can be searched with `use:speedSearch`. Which list the letters
 * go to is decided when the first one is typed: the list holding the focus,
 * else the one last clicked in, else the first one on the screen. A list that
 * already has a filter field names it, and the letters go there instead of
 * into the floating box, so there is one query and not two.
 *
 * A text matches when every word of the query is in it, or when the word's
 * letters start its words in order: `fb` finds `feature/BUG-098`, `cR` finds
 * `CommitRows`. Case is ignored.
 */

/** `[start, end)` of a stretch of text a query matched. */
export type Range = [number, number];

function words(query: string): string[] {
	return query.trim().toLowerCase().split(/\s+/).filter(Boolean);
}

const LETTER = /\p{L}/u;
const ALNUM = /[\p{L}\p{N}]/u;
const LOWER = /\p{Ll}/u;
const UPPER = /\p{Lu}/u;

/**
 * Whether a word starts at `i`: the first character, one after a separator, a
 * capital after a lowercase letter, or the turn from letters to digits.
 */
function startsWord(text: string, i: number): boolean {
	if (i === 0) return true;
	const before = text[i - 1];
	const here = text[i];
	if (!ALNUM.test(before)) return true;
	if (LOWER.test(before) && UPPER.test(here)) return true;
	return LETTER.test(before) !== LETTER.test(here);
}

/**
 * Where `word`'s letters fall when each one starts a word of `text` or
 * follows the letter matched before it, or null. A position that has failed
 * once for a letter fails again, which keeps a long name from going
 * exponential.
 */
function humps(text: string, lower: string, word: string): number[] | null {
	const n = text.length;
	const failed = new Set<number>();
	const at: number[] = [];

	function from(letter: number, start: number): boolean {
		if (letter === word.length) return true;
		for (let t = start; t < n; t += 1) {
			if (lower[t] !== word[letter]) continue;
			const follows = letter > 0 && t === start;
			if (!follows && !startsWord(text, t)) continue;
			const key = letter * (n + 1) + t;
			if (failed.has(key)) continue;
			at.push(t);
			if (from(letter + 1, t + 1)) return true;
			at.pop();
			failed.add(key);
		}
		return false;
	}

	return from(0, 0) ? at : null;
}

/**
 * The stretches of `text` that `query` matched, or null when it does not
 * match. An empty query matches everything and marks nothing.
 */
export function match(text: string, query: string): Range[] | null {
	const asked = words(query);
	if (asked.length === 0) return [];

	const lower = text.toLowerCase();
	const hit = new Array<boolean>(text.length).fill(false);
	for (const word of asked) {
		const at = lower.indexOf(word);
		if (at !== -1) {
			for (let i = at; i < at + word.length; i += 1) hit[i] = true;
			continue;
		}
		const found = humps(text, lower, word);
		if (found === null) return null;
		for (const i of found) hit[i] = true;
	}

	const ranges: Range[] = [];
	for (let i = 0; i < hit.length; i += 1) {
		if (!hit[i]) continue;
		const last = ranges[ranges.length - 1];
		if (last && last[1] === i) last[1] = i + 1;
		else ranges.push([i, i + 1]);
	}
	return ranges;
}

/** What a row is searched by: one text, or several, of which any may be missing. */
export type Searchable = string | null | undefined | readonly (string | null | undefined)[];

function joined(text: Searchable): string {
	if (typeof text === 'string') return text;
	if (text == null) return '';
	return text.filter((part) => part != null && part !== '').join('\n');
}

/** One list's query, and what it narrows. */
export interface Search {
	readonly query: string;
	/** Something has been typed. */
	readonly active: boolean;
	set(query: string): void;
	clear(): void;
	test(text: Searchable): boolean;
	/** `items` narrowed to the ones that match; `items` itself when nothing is typed. */
	filter<T>(items: readonly T[], text: (item: T) => Searchable): T[];
	/** Where to mark `text`, for drawing it. */
	ranges(text: string): Range[];
}

export function createSearch(initial = ''): Search {
	let query = $state(initial);

	return {
		get query() {
			return query;
		},
		get active() {
			return query.trim() !== '';
		},
		set(next: string) {
			query = next;
		},
		clear() {
			query = '';
		},
		test(text: Searchable) {
			return match(joined(text), query) !== null;
		},
		filter<T>(items: readonly T[], text: (item: T) => Searchable): T[] {
			if (query.trim() === '') return items as T[];
			return items.filter((item) => match(joined(text(item)), query) !== null);
		},
		ranges(text: string) {
			return match(text, query) ?? [];
		}
	};
}

export interface SpeedSearchOptions {
	/** The query the box types into. A list with its own `field` needs none. */
	search?: Search;
	/** What is being searched, for the box: "branches". */
	label?: string;
	/**
	 * The list's own filter field. When there is one the first letter is typed
	 * into it and the field takes over; no box is drawn.
	 */
	field?: () => HTMLInputElement | null | undefined;
	/**
	 * Letters the screen already answers to while nothing has been typed —
	 * `jk` where they step through the list. Once a search is open they are
	 * typed like any other letter.
	 */
	reserve?: string;
	/** Enter while searching: open the first match. Without it Enter is left alone. */
	onenter?: () => void;
	/**
	 * The arrow keys while searching, for a list that jumps to its matches
	 * rather than narrowing to them: the graph cannot drop rows without
	 * breaking its lanes.
	 */
	onstep?: (by: 1 | -1) => void;
	/** `[found, total]` for the box; nothing is shown without it. */
	count?: () => [number, number];
	/**
	 * False for a list that is searched only once it has been focused or
	 * clicked in, never as the screen's first list.
	 */
	primary?: boolean;
}

interface Entry {
	node: HTMLElement;
	options: SpeedSearchOptions;
}

const entries: Entry[] = [];
// Raw: an entry is compared by identity, and a deep proxy of it would not be it.
let open = $state.raw<Entry | null>(null);
/** Where the pointer last went down, to tell which list was clicked in last. */
let lastPressed: Element | null = null;
let installed = false;

/** The search the box is showing, for the box. */
export const speed = {
	get open(): { node: HTMLElement; options: SpeedSearchOptions } | null {
		return open;
	},
	close
};

function close(): void {
	if (open === null) return;
	const was = open;
	open = null;
	was.options.search?.clear();
}

function live(entry: Entry): boolean {
	return entry.node.isConnected && entry.node.closest('[hidden], [inert], [aria-hidden="true"]') === null;
}

function editable(target: Element | null): boolean {
	if (target === null) return false;
	if (target.closest('input, textarea, select, [contenteditable=""], [contenteditable="true"], .xterm')) {
		// A checkbox or a button that happens to be an input is not somewhere to type.
		if (target instanceof HTMLInputElement) {
			return !['checkbox', 'radio', 'button', 'submit', 'reset', 'range', 'color', 'file'].includes(
				target.type
			);
		}
		return true;
	}
	return false;
}

/** The innermost of `found`: a list inside another list wins. */
function innermost(found: Entry[]): Entry | null {
	return found.find((entry) => !found.some((other) => other !== entry && entry.node.contains(other.node))) ?? null;
}

/**
 * The list typed letters go to. With a modal dialog open only a list inside
 * it is a candidate; behind one nothing is reachable.
 */
function choose(target: Element | null): Entry | null {
	const dialogs = document.querySelectorAll('[aria-modal="true"]');
	const modal = dialogs.length > 0 ? dialogs[dialogs.length - 1] : null;
	const candidates = entries.filter((entry) => live(entry) && (modal === null || modal.contains(entry.node)));
	if (candidates.length === 0) return null;

	for (const place of [target, lastPressed]) {
		if (place === null || !place.isConnected) continue;
		const holding = innermost(candidates.filter((entry) => entry.node.contains(place)));
		if (holding) return holding;
	}

	const first = candidates
		.filter((entry) => entry.options.primary !== false)
		.sort((a, b) => (a.node.compareDocumentPosition(b.node) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1));
	return first[0] ?? null;
}

function typeInto(field: HTMLInputElement, letter: string): void {
	field.focus();
	const end = field.value.length;
	field.setRangeText(letter, end, end, 'end');
	field.dispatchEvent(new Event('input', { bubbles: true }));
}

function consume(event: KeyboardEvent): void {
	event.preventDefault();
	event.stopPropagation();
}

function onkeydown(event: KeyboardEvent): void {
	if (event.defaultPrevented || event.isComposing) return;
	if (event.ctrlKey || event.metaKey || event.altKey) return;
	const target = event.target instanceof Element ? event.target : null;
	if (editable(target)) return;
	// A menu takes its own keys: arrows, Enter, and its first letters.
	if (document.querySelector('[role="menu"]')) return;

	if (open !== null && !live(open)) close();

	if (open !== null) {
		const { search, onenter, onstep } = open.options as SpeedSearchOptions & { search: Search };
		switch (event.key) {
			case 'Escape':
				consume(event);
				close();
				return;
			case 'Backspace': {
				consume(event);
				const next = search.query.slice(0, -1);
				if (next === '') close();
				else search.set(next);
				return;
			}
			case 'Enter':
				if (!onenter) return;
				consume(event);
				onenter();
				close();
				return;
			case 'ArrowDown':
			case 'ArrowUp':
				if (!onstep) return;
				consume(event);
				onstep(event.key === 'ArrowDown' ? 1 : -1);
				return;
		}
		if (event.key.length === 1) {
			consume(event);
			search.set(search.query + event.key);
		}
		return;
	}

	// A space presses the focused button; it never starts a search.
	if (event.key.length !== 1 || event.key === ' ') return;
	const entry = choose(target);
	if (entry === null) return;
	if (entry.options.reserve?.includes(event.key)) return;

	const { field, search } = entry.options;
	const into = field?.();
	if (into) {
		consume(event);
		typeInto(into, event.key);
	} else if (search) {
		consume(event);
		open = entry;
		search.set(event.key);
	}
}

function onpointerdown(event: PointerEvent): void {
	const target = event.target instanceof Element ? event.target : null;
	lastPressed = target;
	// Clicking away ends the search, as moving the focus does in IntelliJ.
	// A click in the list keeps it: picking a match is what it was for.
	if (open !== null && target !== null && !open.node.contains(target) && !target.closest('.speed-search')) {
		close();
	}
}

function install(): void {
	if (installed || typeof window === 'undefined') return;
	installed = true;
	// Capture, so a letter that starts a search never reaches a screen's own
	// `j`/`k` handler as well.
	window.addEventListener('keydown', onkeydown, true);
	window.addEventListener('pointerdown', onpointerdown, true);
}

/** Make a list searchable by typing. */
export function speedSearch(node: HTMLElement, options: SpeedSearchOptions) {
	install();
	const entry: Entry = { node, options };
	entries.push(entry);
	return {
		update(next: SpeedSearchOptions) {
			entry.options = next;
		},
		destroy() {
			entries.splice(entries.indexOf(entry), 1);
			if (open === entry) close();
			// A query typed into the box goes with the list; one typed into the
			// list's own field is the field's to keep.
			else if (!entry.options.field) entry.options.search?.clear();
		}
	};
}

/** For tests: forget every list and the last click. */
export function resetSpeedSearch(): void {
	entries.length = 0;
	open = null;
	lastPressed = null;
}
