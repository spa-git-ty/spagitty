// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Which keys are not the shell's (FEAT-130).
 *
 * While the terminal has the focus, every key goes to the shell. A terminal
 * that kept `Ctrl+F` for the application could not run a program that uses
 * it, and every terminal program uses something. Two keys are kept, both with
 * Shift so no program is waiting for them: the palette, and the way out. Copy
 * and paste are kept as well, with Shift on Windows and Linux where `Ctrl+C`
 * is the shell's interrupt, and with ⌘ on macOS where it never was.
 *
 * The same table is read in two places: by the emulator, to decide what it
 * must not swallow, and by the layout, to decide what it may act on while
 * the focus is in a terminal.
 */

export type Kept = 'palette' | 'leave' | 'copy' | 'paste';

export interface Pressed {
	key: string;
	/** The physical key, which is what a backquote is found by on every layout. */
	code: string;
	ctrl: boolean;
	shift: boolean;
	alt: boolean;
	meta: boolean;
}

/** What a key is kept for, or null when it is the shell's. */
export function kept(pressed: Pressed, mac: boolean): Kept | null {
	const primary = mac ? pressed.meta : pressed.ctrl;
	if (pressed.alt) return null;

	if (mac) {
		// ⌘C and ⌘V were never the shell's.
		if (pressed.meta && !pressed.shift && !pressed.ctrl) {
			if (pressed.key.toLowerCase() === 'c') return 'copy';
			if (pressed.key.toLowerCase() === 'v') return 'paste';
		}
	}
	if (!primary || !pressed.shift) return null;

	if (pressed.code === 'Backquote') return 'leave';
	switch (pressed.key.toLowerCase()) {
		case 'p':
			return 'palette';
		case 'c':
			return mac ? null : 'copy';
		case 'v':
			return mac ? null : 'paste';
	}
	return null;
}

/** `Ctrl+`` (⌘` on macOS), with no Shift: open Terminal, or go back from it. */
export function isToggle(pressed: Pressed, mac: boolean): boolean {
	const primary = mac ? pressed.meta : pressed.ctrl;
	return primary && !pressed.shift && !pressed.alt && pressed.code === 'Backquote';
}

/** A keyboard event as the table reads it. */
export function pressedOf(event: KeyboardEvent): Pressed {
	return {
		key: event.key,
		code: event.code,
		ctrl: event.ctrlKey,
		shift: event.shiftKey,
		alt: event.altKey,
		meta: event.metaKey
	};
}
