// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The keys Files reads by (FEAT-131).
 *
 * Neovim-shaped, and limited to reading: move, search, step through blame,
 * open the peek, copy where you are. No key enters a mode that types into the
 * file, because there is no such mode — Files reads git's objects and has
 * nothing to write to.
 *
 * Kept apart from the component so the table is tested as a table. Two of the
 * commands are two keys long (`gg`, `]b`, `[b`), which is the only state
 * there is: the key waited on.
 */

import type { SearchOptions } from '../types';

export type Action =
	| { kind: 'move'; by: number }
	/** Half a screen, as `Ctrl+d` and `Ctrl+u` go. `by` is 1 or -1. */
	| { kind: 'half'; by: number }
	| { kind: 'top' }
	| { kind: 'bottom' }
	/** Open the search field. `backward` is `?`. */
	| { kind: 'search'; backward: boolean }
	/** The next match, or with `reverse` the one before. */
	| { kind: 'next'; reverse: boolean }
	/** The next blame block, or with `by: -1` the one before. */
	| { kind: 'block'; by: number }
	| { kind: 'blame' }
	| { kind: 'peek' }
	| { kind: 'close' }
	| { kind: 'copy' };

/** What the key machine remembers between keys: the first key of a pair. */
export type Pending = 'g' | ']' | '[' | null;

export interface Pressed {
	key: string;
	ctrl?: boolean;
	meta?: boolean;
	alt?: boolean;
}

export interface Step {
	pending: Pending;
	/** What to do, or nothing: an unknown key, or the first key of a pair. */
	action: Action | null;
	/** True when the key was one of ours, and the page must not also act on it. */
	handled: boolean;
}

const NONE: Step = { pending: null, action: null, handled: false };

function act(action: Action): Step {
	return { pending: null, action, handled: true };
}

/**
 * One key, given the key waited on.
 *
 * A second key that does not finish its pair is dropped with the first, and
 * is not read as a command of its own: `g` then `j` moves nowhere, as in the
 * editor these keys come from.
 */
export function step(pending: Pending, pressed: Pressed): Step {
	const { key } = pressed;
	// Alt and the platform key belong to the application and the window.
	if (pressed.alt || pressed.meta) return NONE;

	if (pressed.ctrl) {
		if (key === 'd') return act({ kind: 'half', by: 1 });
		if (key === 'u') return act({ kind: 'half', by: -1 });
		return NONE;
	}

	if (pending === 'g') return key === 'g' ? act({ kind: 'top' }) : { ...NONE, handled: true };
	if (pending === ']') return key === 'b' ? act({ kind: 'block', by: 1 }) : { ...NONE, handled: true };
	if (pending === '[') return key === 'b' ? act({ kind: 'block', by: -1 }) : { ...NONE, handled: true };

	switch (key) {
		case 'j':
		case 'ArrowDown':
			return act({ kind: 'move', by: 1 });
		case 'k':
		case 'ArrowUp':
			return act({ kind: 'move', by: -1 });
		case 'g':
		case ']':
		case '[':
			return { pending: key, action: null, handled: true };
		case 'G':
			return act({ kind: 'bottom' });
		case '/':
			return act({ kind: 'search', backward: false });
		case '?':
			return act({ kind: 'search', backward: true });
		case 'n':
			return act({ kind: 'next', reverse: false });
		case 'N':
			return act({ kind: 'next', reverse: true });
		case 'b':
			return act({ kind: 'blame' });
		case 'Enter':
			return act({ kind: 'peek' });
		case 'Escape':
			return act({ kind: 'close' });
		case 'y':
			return act({ kind: 'copy' });
	}
	return NONE;
}

/** A place a search found: a line (0-based) and the match's ends in it. */
export interface Match {
	line: number;
	start: number;
	end: number;
}

/**
 * Alt with R, W or C in a search field: which option it flips (FEAT-151),
 * or null for any other key.
 */
export function optionKey(event: KeyboardEvent): keyof SearchOptions | null {
	if (!event.altKey || event.ctrlKey || event.metaKey) return null;
	switch (event.code) {
		case 'KeyR':
			return 'regex';
		case 'KeyW':
			return 'word';
		case 'KeyC':
			return 'case';
	}
	return null;
}

/** Text as typed, smart about case: what `/` has always done. */
export const PLAIN: SearchOptions = { regex: false, word: false, case: false };

/**
 * Does `query` ask for its case to count? Always with *Match case*; without
 * it, only when it has a capital — an escape such as `\S` in a regular
 * expression is not one. Mirrors `files::case_counts`.
 */
export function caseCounts(query: string, options: SearchOptions): boolean {
	if (options.case) return true;
	const plain = options.regex ? query.replace(/\\./g, '') : query;
	return plain !== plain.toLowerCase();
}

/**
 * What was typed, as the expression a search matches with (FEAT-151), or
 * why it is not one. Mirrors `files::pattern`.
 */
export function patternOf(query: string, options: SearchOptions = PLAIN): RegExp | string {
	const body = options.regex ? query : query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
	const wrapped = options.word ? `\\b(?:${body})\\b` : body;
	try {
		return new RegExp(wrapped, caseCounts(query, options) ? 'g' : 'gi');
	} catch (error) {
		return error instanceof Error ? error.message : String(error);
	}
}

/** Where `pattern` matches in `text`, start and end, empty matches left out. */
function spans(text: string, pattern: RegExp): [number, number][] {
	const out: [number, number][] = [];
	pattern.lastIndex = 0;
	for (let found = pattern.exec(text); found; found = pattern.exec(text)) {
		if (found[0].length === 0) {
			pattern.lastIndex += 1;
			continue;
		}
		out.push([found.index, found.index + found[0].length]);
	}
	return out;
}

/**
 * The next place `query` is found, from `from` in the direction asked,
 * wrapping at either end. Null when it is nowhere, or when a regular
 * expression does not parse.
 *
 * By default the query is text and case is ignored unless it has a capital
 * in it, which is what `/` does with `smartcase` set and what most people
 * expect either way; `options` can make it a regular expression, a whole
 * word, or case-sensitive (FEAT-151). `from.col` is where the cursor is: a
 * match starting there is the one you are on, and the next one is past it.
 */
export function find(
	lines: readonly string[],
	query: string,
	from: { line: number; col: number },
	backward: boolean,
	options: SearchOptions = PLAIN
): Match | null {
	if (query === '' || lines.length === 0) return null;
	const pattern = patternOf(query, options);
	if (typeof pattern === 'string') return null;
	const count = lines.length;

	// The cursor's own line first, on the far side of the cursor; then every
	// other line; then the cursor's line again, on the near side.
	for (let turn = 0; turn <= count; turn++) {
		const index = (((from.line + (backward ? -turn : turn)) % count) + count) % count;
		const found = spans(lines[index], pattern);
		let hit: [number, number] | undefined;
		if (turn === 0) {
			hit = backward ? found.filter(([start]) => start < from.col).at(-1) : found.find(([start]) => start > from.col);
		} else if (turn === count) {
			// Only the part the first look did not cover.
			hit = backward ? found.filter(([start]) => start >= from.col).at(-1) : found.find(([start]) => start <= from.col);
		} else {
			hit = backward ? found.at(-1) : found[0];
		}
		if (hit) return { line: index, start: hit[0], end: hit[1] };
	}
	return null;
}

/**
 * The line a blame block starts on, going `by` blocks from `line`.
 *
 * A block is a run of lines from one commit. `commits[i]` is line `i`'s
 * commit. Stays put at either end rather than wrapping: the first block has
 * none before it.
 */
export function blockFrom(commits: readonly number[], line: number, by: number): number {
	if (commits.length === 0) return line;
	let at = Math.min(Math.max(line, 0), commits.length - 1);
	const startOf = (index: number) => {
		let start = index;
		while (start > 0 && commits[start - 1] === commits[index]) start--;
		return start;
	};

	if (by > 0) {
		let next = at + 1;
		while (next < commits.length && commits[next] === commits[at]) next++;
		return next < commits.length ? next : at;
	}
	const start = startOf(at);
	// From inside a block, back is its own first line; from its first line,
	// the block before.
	if (start < at) return start;
	at = start;
	return at > 0 ? startOf(at - 1) : at;
}
