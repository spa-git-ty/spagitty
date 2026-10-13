// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The builder's keys (FEAT-133).
 *
 * A recipe is built without the pointer: a block's own letter adds it, `j`
 * and `k` walk the stack, Alt with the arrows reorders, Delete removes,
 * Ctrl+Enter runs and Ctrl+Z takes the last edit back.
 *
 * One table, tested as one. None of it fires while a field has the focus:
 * a letter typed into a commit message is a letter.
 */

import { byKey } from './catalogue';
import type { BlockId } from './types';

export type KeyAction =
	| { kind: 'add'; block: BlockId }
	| { kind: 'down' }
	| { kind: 'up' }
	| { kind: 'moveDown' }
	| { kind: 'moveUp' }
	| { kind: 'remove' }
	| { kind: 'run' }
	| { kind: 'undo' };

export interface Pressed {
	key: string;
	ctrl: boolean;
	meta: boolean;
	alt: boolean;
	shift: boolean;
}

/** What a key does in the builder, or null when it is not the builder's. */
export function keyAction(pressed: Pressed, mac: boolean): KeyAction | null {
	const primary = mac ? pressed.meta : pressed.ctrl;
	const other = mac ? pressed.ctrl : pressed.meta;
	if (other) return null;

	if (primary) {
		if (pressed.alt) return null;
		if (pressed.key === 'Enter') return { kind: 'run' };
		if (pressed.key.toLowerCase() === 'z' && !pressed.shift) return { kind: 'undo' };
		return null;
	}
	if (pressed.alt) {
		if (pressed.key === 'ArrowUp') return { kind: 'moveUp' };
		if (pressed.key === 'ArrowDown') return { kind: 'moveDown' };
		return null;
	}

	switch (pressed.key) {
		case 'j':
		case 'ArrowDown':
			return { kind: 'down' };
		case 'k':
		case 'ArrowUp':
			return { kind: 'up' };
		case 'Delete':
		case 'Backspace':
			return { kind: 'remove' };
	}
	// `j` and `k` walk the stack, so a block's key is its capital: Shift, or
	// the bare letter for every block whose letter is not one of those two.
	if (pressed.key.length !== 1) return null;
	const block = byKey(pressed.key);
	return block ? { kind: 'add', block: block.id } : null;
}

/** True when what has the focus takes text: its keys are its own. */
export function typing(target: EventTarget | null): boolean {
	const element = target as HTMLElement | null;
	if (!element || typeof element.closest !== 'function') return false;
	return element.closest('input, textarea, select, [contenteditable="true"], [role="menu"], [role="listbox"]') !== null;
}

/** The keys, as the line under the stack lists them. */
export function keyHints(mac: boolean): { keys: string; what: string }[] {
	const primary = mac ? '⌘' : 'Ctrl';
	return [
		{ keys: 'S P C …', what: 'add a block' },
		{ keys: 'J K', what: 'move' },
		{ keys: 'Alt ↑↓', what: 'reorder' },
		{ keys: 'Del', what: 'remove' },
		{ keys: `${primary} ↵`, what: 'run' },
		{ keys: `${primary} Z`, what: 'undo' }
	];
}
