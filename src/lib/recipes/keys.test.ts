// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { CATALOGUE } from './catalogue';
import { keyAction, keyHints, typing, type Pressed } from './keys';

function key(key: string, mods: Partial<Omit<Pressed, 'key'>> = {}): Pressed {
	return { key, ctrl: false, meta: false, alt: false, shift: false, ...mods };
}

describe('the builder’s keys', () => {
	it('adds a block with its own letter, in either case', () => {
		expect(keyAction(key('s'), false)).toEqual({ kind: 'add', block: 'stash' });
		expect(keyAction(key('S', { shift: true }), false)).toEqual({ kind: 'add', block: 'stash' });
		expect(keyAction(key('?', { shift: true }), false)).toEqual({ kind: 'add', block: 'ask' });
		// Every block can be added from the keyboard.
		for (const block of CATALOGUE) {
			expect(keyAction(key(block.key, { shift: true }), false)).toEqual({ kind: 'add', block: block.id });
		}
	});

	it('walks the stack with j and k and with the arrows', () => {
		expect(keyAction(key('j'), false)).toEqual({ kind: 'down' });
		expect(keyAction(key('k'), false)).toEqual({ kind: 'up' });
		expect(keyAction(key('ArrowDown'), false)).toEqual({ kind: 'down' });
		expect(keyAction(key('ArrowUp'), false)).toEqual({ kind: 'up' });
	});

	it('reorders with Alt and the arrows', () => {
		expect(keyAction(key('ArrowUp', { alt: true }), false)).toEqual({ kind: 'moveUp' });
		expect(keyAction(key('ArrowDown', { alt: true }), false)).toEqual({ kind: 'moveDown' });
		// Alt with a letter is not a block: it is somebody else's key.
		expect(keyAction(key('s', { alt: true }), false)).toBeNull();
	});

	it('removes with Delete or Backspace', () => {
		expect(keyAction(key('Delete'), false)).toEqual({ kind: 'remove' });
		expect(keyAction(key('Backspace'), false)).toEqual({ kind: 'remove' });
	});

	it('runs with Ctrl+Enter and takes an edit back with Ctrl+Z', () => {
		expect(keyAction(key('Enter', { ctrl: true }), false)).toEqual({ kind: 'run' });
		expect(keyAction(key('z', { ctrl: true }), false)).toEqual({ kind: 'undo' });
		expect(keyAction(key('Enter'), false)).toBeNull();
		// Ctrl with a block's letter is the application's, not a block.
		expect(keyAction(key('s', { ctrl: true }), false)).toBeNull();
		expect(keyAction(key('c', { ctrl: true }), false)).toBeNull();
		// Redo is not offered, so its key is left alone.
		expect(keyAction(key('z', { ctrl: true, shift: true }), false)).toBeNull();
	});

	it('on macOS uses ⌘ where the others use Ctrl', () => {
		expect(keyAction(key('Enter', { meta: true }), true)).toEqual({ kind: 'run' });
		expect(keyAction(key('z', { meta: true }), true)).toEqual({ kind: 'undo' });
		expect(keyAction(key('Enter', { ctrl: true }), true)).toBeNull();
		expect(keyAction(key('z', { meta: true }), false)).toBeNull();
	});

	it('does nothing for a key that is nobody’s', () => {
		expect(keyAction(key('z'), false)).toBeNull();
		expect(keyAction(key('1'), false)).toBeNull();
		expect(keyAction(key('Tab'), false)).toBeNull();
		expect(keyAction(key('Escape'), false)).toBeNull();
	});

	/** A letter typed into a commit message is a letter. */
	it('leaves every key alone while a field has the focus', () => {
		const field = document.createElement('input');
		const area = document.createElement('textarea');
		const choice = document.createElement('select');
		const menu = document.createElement('div');
		menu.setAttribute('role', 'listbox');
		const row = document.createElement('span');
		menu.appendChild(row);
		const card = document.createElement('div');

		for (const within of [field, area, choice, menu, row]) expect(typing(within)).toBe(true);
		expect(typing(card)).toBe(false);
		expect(typing(null)).toBe(false);
		expect(typing(window)).toBe(false);
	});

	it('lists its keys under the stack', () => {
		expect(keyHints(false).map((hint) => `${hint.keys} ${hint.what}`)).toEqual([
			'S P C … add a block',
			'J K move',
			'Alt ↑↓ reorder',
			'Del remove',
			'Ctrl ↵ run',
			'Ctrl Z undo'
		]);
		expect(keyHints(true)[4].keys).toBe('⌘ ↵');
	});
});
