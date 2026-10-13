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
 * The next place `query` is found, from `from` in the direction asked,
 * wrapping at either end. Null when it is nowhere.
 *
 * Case is ignored unless the query has a capital in it, which is what `/`
 * does with `smartcase` set and what most people expect either way.
 * `from.col` is where the cursor is: a match starting there is the one you
 * are on, and the next one is past it.
 */
export function find(
	lines: readonly string[],
	query: string,
	from: { line: number; col: number },
	backward: boolean
): Match | null {
	if (query === '' || lines.length === 0) return null;
	const exact = query !== query.toLowerCase();
	const needle = exact ? query : query.toLowerCase();
	const read = (index: number) => (exact ? lines[index] : lines[index].toLowerCase());
	const count = lines.length;

	// The cursor's own line first, on the far side of the cursor; then every
	// other line; then the cursor's line again, on the near side.
	for (let turn = 0; turn <= count; turn++) {
		const index = (((from.line + (backward ? -turn : turn)) % count) + count) % count;
		const text = read(index);
		let at: number;
		if (turn === 0) {
			at = backward
				? from.col > 0
					? text.lastIndexOf(needle, from.col - 1)
					: -1
				: text.indexOf(needle, from.col + 1);
		} else if (turn === count) {
			at = backward ? text.lastIndexOf(needle) : text.indexOf(needle);
			// Only the part the first look did not cover.
			if (at !== -1 && (backward ? at < from.col : at > from.col)) at = -1;
		} else {
			at = backward ? text.lastIndexOf(needle) : text.indexOf(needle);
		}
		if (at !== -1) return { line: index, start: at, end: at + needle.length };
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
