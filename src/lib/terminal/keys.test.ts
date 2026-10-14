// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { isToggle, kept, pressedOf, type Pressed } from './keys';

function key(key: string, mods: Partial<Omit<Pressed, 'key' | 'code'>> = {}, code?: string): Pressed {
	return {
		key,
		code: code ?? (key.length === 1 ? `Key${key.toUpperCase()}` : key),
		ctrl: false,
		shift: false,
		alt: false,
		meta: false,
		...mods
	};
}

const backquote = (mods: Partial<Omit<Pressed, 'key' | 'code'>> = {}) => key('`', mods, 'Backquote');

describe('while the terminal has the focus', () => {
	it('gives every plain key and every Ctrl key to the shell', () => {
		for (const mac of [false, true]) {
			expect(kept(key('a'), mac)).toBeNull();
			expect(kept(key('Enter'), mac)).toBeNull();
			// The application's own shortcuts are the shell's here: find, the
			// palette's `Ctrl+K`, a new tab, and the interrupt.
			for (const letter of ['f', 'k', 't', 'c', 'v', 'p', 'r', 'd', 'w']) {
				expect(kept(key(letter, { ctrl: true }), mac), `Ctrl+${letter}`).toBeNull();
			}
		}
	});

	it('keeps Ctrl+Shift+P for the palette and Ctrl+Shift+` for the way out', () => {
		expect(kept(key('P', { ctrl: true, shift: true }), false)).toBe('palette');
		expect(kept(backquote({ ctrl: true, shift: true }), false)).toBe('leave');
	});

	/** A backquote with Shift is `~` on a US layout and something else on others. */
	it('finds the backquote by the key pressed, whatever the layout prints', () => {
		expect(kept(key('~', { ctrl: true, shift: true }, 'Backquote'), false)).toBe('leave');
		expect(kept(key('¬', { ctrl: true, shift: true }, 'Backquote'), false)).toBe('leave');
		// A `~` typed from another key is not it.
		expect(kept(key('~', { ctrl: true, shift: true }, 'Digit1'), false)).toBeNull();
	});

	it('copies and pastes with Shift on Windows and Linux, where Ctrl+C is the interrupt', () => {
		expect(kept(key('C', { ctrl: true, shift: true }), false)).toBe('copy');
		expect(kept(key('V', { ctrl: true, shift: true }), false)).toBe('paste');
		expect(kept(key('c', { ctrl: true }), false)).toBeNull();
		// ⌘ is not a modifier Windows and Linux keep anything for.
		expect(kept(key('c', { meta: true }), false)).toBeNull();
	});

	it('on macOS keeps ⌘C and ⌘V, and ⇧⌘P and ⇧⌘`', () => {
		expect(kept(key('c', { meta: true }), true)).toBe('copy');
		expect(kept(key('v', { meta: true }), true)).toBe('paste');
		expect(kept(key('P', { meta: true, shift: true }), true)).toBe('palette');
		expect(kept(backquote({ meta: true, shift: true }), true)).toBe('leave');
		// Ctrl is the shell's on macOS, with Shift or without.
		expect(kept(key('c', { ctrl: true }), true)).toBeNull();
		expect(kept(key('P', { ctrl: true, shift: true }), true)).toBeNull();
		expect(kept(key('C', { meta: true, shift: true }), true)).toBeNull();
		expect(kept(key('V', { meta: true, shift: true }), true)).toBeNull();
		expect(kept(key('c', { meta: true, ctrl: true }), true)).toBeNull();
	});

	it('keeps nothing that has Alt in it', () => {
		expect(kept(key('P', { ctrl: true, shift: true, alt: true }), false)).toBeNull();
		expect(kept(backquote({ ctrl: true, shift: true, alt: true }), false)).toBeNull();
		expect(kept(key('c', { meta: true, alt: true }), true)).toBeNull();
	});

	it('keeps no other key with Ctrl and Shift', () => {
		expect(kept(key('F', { ctrl: true, shift: true }), false)).toBeNull();
		expect(kept(key('T', { ctrl: true, shift: true }), false)).toBeNull();
	});
});

describe('Ctrl+`', () => {
	it('is the key to Terminal and back: Ctrl+` and, on macOS, ⌘`', () => {
		expect(isToggle(backquote({ ctrl: true }), false)).toBe(true);
		expect(isToggle(backquote({ meta: true }), true)).toBe(true);
		expect(isToggle(backquote({ ctrl: true }), true)).toBe(false);
		expect(isToggle(backquote({ meta: true }), false)).toBe(false);
	});

	it('is not the key with Shift or Alt, or a backquote alone', () => {
		expect(isToggle(backquote(), false)).toBe(false);
		expect(isToggle(backquote({ ctrl: true, shift: true }), false)).toBe(false);
		expect(isToggle(backquote({ ctrl: true, alt: true }), false)).toBe(false);
		expect(isToggle(key('`', { ctrl: true }, 'KeyQ'), false)).toBe(false);
	});
});

describe('a keyboard event', () => {
	it('is read as the table reads it', () => {
		const event = new KeyboardEvent('keydown', {
			key: 'P',
			code: 'KeyP',
			ctrlKey: true,
			shiftKey: true
		});

		expect(pressedOf(event)).toEqual({
			key: 'P',
			code: 'KeyP',
			ctrl: true,
			shift: true,
			alt: false,
			meta: false
		});
		expect(kept(pressedOf(event), false)).toBe('palette');
	});
});
