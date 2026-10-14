// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What the Terminal screen asks of a terminal emulator (FEAT-130).
 *
 * The emulator is xterm.js (`xterm.ts`). It is behind this interface so that
 * the store and the screen — which terminal is shown, what is written to it,
 * what happens when it is closed — are tested without a canvas, with an
 * emulator that records what it was given.
 */

import type { Link } from './links';

export interface Emulator {
	/** The element the terminal draws in. It is moved into the screen, and out of it. */
	readonly element: HTMLElement;
	/** Bytes from the program, as they were read. */
	write(bytes: Uint8Array): void;
	/** Take the size of its element, and say what that is in cells. */
	fit(): { cols: number; rows: number };
	focus(): void;
	/** The text selected, for copying. */
	selection(): string;
	/** Text as if typed, sent as a paste where the program asked for that. */
	paste(text: string): void;
	dispose(): void;
}

export interface EmulatorHooks {
	/** Keystrokes, and anything else the terminal sends the program. */
	onData(data: string): void;
	/** The links on a line of output: only the ones that have been checked. */
	links(line: string): Promise<Link[]>;
	open(link: Link): void;
}

export type MakeEmulator = (hooks: EmulatorHooks) => Emulator;
