// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { blockFrom, find, step, type Action, type Pending } from './keys';

/** Press keys one after another, collecting what each did. */
function press(...keys: (string | { key: string; ctrl?: boolean; alt?: boolean; meta?: boolean })[]) {
	let pending: Pending = null;
	const actions: (Action | null)[] = [];
	let handled = false;
	for (const key of keys) {
		const taken = step(pending, typeof key === 'string' ? { key } : key);
		pending = taken.pending;
		actions.push(taken.action);
		handled = taken.handled;
	}
	return { actions, pending, handled, last: actions[actions.length - 1] };
}

describe('the keys Files reads by', () => {
	it('is the table the item gives', () => {
		const table: [Parameters<typeof press>, Action][] = [
			[['j'], { kind: 'move', by: 1 }],
			[['k'], { kind: 'move', by: -1 }],
			[['ArrowDown'], { kind: 'move', by: 1 }],
			[['ArrowUp'], { kind: 'move', by: -1 }],
			[[{ key: 'd', ctrl: true }], { kind: 'half', by: 1 }],
			[[{ key: 'u', ctrl: true }], { kind: 'half', by: -1 }],
			[['g', 'g'], { kind: 'top' }],
			[['G'], { kind: 'bottom' }],
			[['/'], { kind: 'search', backward: false }],
			[['?'], { kind: 'search', backward: true }],
			[['n'], { kind: 'next', reverse: false }],
			[['N'], { kind: 'next', reverse: true }],
			[[']', 'b'], { kind: 'block', by: 1 }],
			[['[', 'b'], { kind: 'block', by: -1 }],
			[['b'], { kind: 'blame' }],
			[['Enter'], { kind: 'peek' }],
			[['Escape'], { kind: 'close' }],
			[['y'], { kind: 'copy' }]
		];

		for (const [keys, action] of table) {
			const pressed = press(...keys);
			expect(pressed.last, JSON.stringify(keys)).toEqual(action);
			expect(pressed.handled).toBe(true);
			expect(pressed.pending).toBeNull();
		}
	});

	it('waits on the first key of a pair and does nothing yet', () => {
		for (const key of ['g', ']', '[']) {
			const pressed = press(key);
			expect(pressed.last).toBeNull();
			expect(pressed.pending).toBe(key);
			expect(pressed.handled).toBe(true);
		}
	});

	/** As in the editor these keys come from: `g` then `j` moves nowhere. */
	it('drops a pair the second key does not finish', () => {
		for (const keys of [
			['g', 'j'],
			[']', 'x'],
			['[', 'G']
		]) {
			const pressed = press(...keys);
			expect(pressed.last, keys.join('')).toBeNull();
			expect(pressed.pending).toBeNull();
		}
		// And the key after that is read fresh.
		expect(press('g', 'x', 'j').last).toEqual({ kind: 'move', by: 1 });
	});

	/**
	 * Files cannot write, and no key pretends otherwise: none of the keys that
	 * enter a typing mode in the editor these come from does anything here.
	 */
	it('has no key that types into the file', () => {
		for (const key of ['i', 'a', 'o', 'O', 'I', 'A', 'c', 'C', 's', 'S', 'x', 'X', 'd', 'D', 'r', 'R', 'p', 'P', 'u', ':', 'v', 'V']) {
			const pressed = press(key);
			expect(pressed.last, key).toBeNull();
			expect(pressed.handled, key).toBe(false);
		}
	});

	it('leaves the application its own shortcuts', () => {
		// Ctrl+F is the Log, Ctrl+P the palette, and Alt and the platform key
		// are never ours.
		for (const key of [
			{ key: 'f', ctrl: true },
			{ key: 'p', ctrl: true },
			{ key: 'j', alt: true },
			{ key: 'j', meta: true },
			{ key: 'c', meta: true }
		]) {
			expect(press(key).handled, JSON.stringify(key)).toBe(false);
		}
	});
});

describe('find', () => {
	const lines = ['fn one() {', '    two();', '    One::two();', '}'];

	it('finds the next place after the cursor, and wraps', () => {
		expect(find(lines, 'two', { line: 0, col: 0 }, false)).toEqual({ line: 1, start: 4, end: 7 });
		expect(find(lines, 'two', { line: 1, col: 4 }, false)).toEqual({ line: 2, start: 9, end: 12 });
		// Past the last one, back to the first.
		expect(find(lines, 'two', { line: 2, col: 9 }, false)).toEqual({ line: 1, start: 4, end: 7 });
	});

	it('goes backwards, and wraps', () => {
		expect(find(lines, 'two', { line: 2, col: 9 }, true)).toEqual({ line: 1, start: 4, end: 7 });
		expect(find(lines, 'two', { line: 1, col: 4 }, true)).toEqual({ line: 2, start: 9, end: 12 });
	});

	it('ignores case until the word has a capital in it', () => {
		expect(find(lines, 'one', { line: 3, col: 0 }, false)).toEqual({ line: 0, start: 3, end: 6 });
		expect(find(lines, 'one', { line: 0, col: 3 }, false)).toEqual({ line: 2, start: 4, end: 7 });
		expect(find(lines, 'One', { line: 0, col: 0 }, false)).toEqual({ line: 2, start: 4, end: 7 });
		expect(find(lines, 'One', { line: 2, col: 4 }, false)).toEqual({ line: 2, start: 4, end: 7 });
	});

	it('finds a second match on the cursor’s own line before leaving it', () => {
		const one = ['a b a b a'];
		expect(find(one, 'a', { line: 0, col: 0 }, false)).toEqual({ line: 0, start: 4, end: 5 });
		expect(find(one, 'a', { line: 0, col: 4 }, false)).toEqual({ line: 0, start: 8, end: 9 });
		expect(find(one, 'a', { line: 0, col: 8 }, false)).toEqual({ line: 0, start: 0, end: 1 });
		expect(find(one, 'a', { line: 0, col: 4 }, true)).toEqual({ line: 0, start: 0, end: 1 });
		expect(find(one, 'a', { line: 0, col: 0 }, true)).toEqual({ line: 0, start: 8, end: 9 });
	});

	it('says nothing when the word is nowhere, or there is no word', () => {
		expect(find(lines, 'three', { line: 0, col: 0 }, false)).toBeNull();
		expect(find(lines, '', { line: 0, col: 0 }, false)).toBeNull();
		expect(find([], 'a', { line: 0, col: 0 }, false)).toBeNull();
	});
});

describe('blockFrom', () => {
	// Lines:      0  1  2  3  4  5  6  7
	const commits = [0, 0, 1, 1, 1, 0, 2, 2];

	it('goes to the first line of the next block', () => {
		expect(blockFrom(commits, 0, 1)).toBe(2);
		expect(blockFrom(commits, 1, 1)).toBe(2);
		expect(blockFrom(commits, 3, 1)).toBe(5);
		expect(blockFrom(commits, 5, 1)).toBe(6);
	});

	it('goes back to its own first line, then to the block before', () => {
		expect(blockFrom(commits, 4, -1)).toBe(2);
		expect(blockFrom(commits, 2, -1)).toBe(0);
		expect(blockFrom(commits, 7, -1)).toBe(6);
		expect(blockFrom(commits, 6, -1)).toBe(5);
	});

	/** The same commit twice is two blocks: a block is a run, not a commit. */
	it('counts a run, not a commit', () => {
		expect(blockFrom(commits, 5, -1)).toBe(2);
	});

	it('stays put at either end', () => {
		expect(blockFrom(commits, 7, 1)).toBe(7);
		expect(blockFrom(commits, 6, 1)).toBe(6);
		expect(blockFrom(commits, 0, -1)).toBe(0);
		expect(blockFrom([], 3, 1)).toBe(3);
	});
});
