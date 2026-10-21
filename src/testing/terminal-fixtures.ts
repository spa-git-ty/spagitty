// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A terminal with no canvas and no program, for the Terminal tests (FEAT-130).
 *
 * Two halves. The **emulator** records what it was given in place of drawing
 * it. The **PTY** stands where the backend's is: each `terminal_open` is kept
 * with the channel its output goes down, so a test prints from the "program"
 * and reads what the screen sent back.
 *
 * Lives outside `src/lib` so it is not counted as first-party code by the
 * coverage floor.
 */

import type { Emulator, EmulatorHooks } from '$lib/terminal/emulator';
import type { TerminalInfo, TerminalSession, TerminalStyle, TerminalWindow } from '$lib/types';

export interface Recorded extends Emulator {
	hooks: EmulatorHooks;
	/** Everything written, decoded. */
	text(): string;
	/** How many writes it took. */
	writes: number;
	fits: number;
	focused: number;
	pasted: string[];
	disposed: boolean;
	/** How many times it took the theme's colours again. */
	rethemed: number;
	/** What the next fit says the element holds. */
	size: { cols: number; rows: number };
}

/** Every emulator made since `reset`, oldest first. */
export const emulators: Recorded[] = [];

export function makeRecorded(hooks: EmulatorHooks): Recorded {
	const element = document.createElement('div');
	element.className = 'xterm fake';
	const decoder = new TextDecoder();
	let written = '';
	const made: Recorded = {
		element,
		hooks,
		writes: 0,
		fits: 0,
		focused: 0,
		pasted: [],
		disposed: false,
		rethemed: 0,
		size: { cols: 132, rows: 40 },
		text: () => written,
		write(bytes) {
			made.writes += 1;
			written += decoder.decode(bytes, { stream: true });
		},
		fit() {
			made.fits += 1;
			return { ...made.size };
		},
		focus() {
			made.focused += 1;
		},
		selection: () => '',
		paste(text) {
			made.pasted.push(text);
		},
		retheme() {
			made.rethemed += 1;
		},
		dispose() {
			made.disposed = true;
			element.remove();
		}
	};
	emulators.push(made);
	return made;
}

/** One `terminal_open`: what was asked, and the channel its output goes down. */
export interface Opened {
	id: number;
	ask: { session: string | null; cwd: string; cols: number; rows: number };
	/** Print from the program. */
	print(text: string): void;
}

export const TMUX: TerminalInfo = {
	mux: { name: 'tmux', version: '3.5a', path: '/usr/bin/tmux', bundled: false },
	shell: '/bin/zsh'
};

export const PSMUX: TerminalInfo = {
	mux: { name: 'psmux', version: '3.3.8', path: 'C:\\app\\multiplexer\\psmux.exe', bundled: true },
	shell: 'C:\\Program Files\\PowerShell\\7\\pwsh.exe'
};

/** The theme's colours as the screen reads them for the multiplexer (FEAT-157). */
export const STYLE: TerminalStyle = {
	ground: '#171514',
	ink: '#ebe4dc',
	muted: '#9a948e',
	line: '#45403d',
	accent: '#f2715a',
	onAccent: '#2a0e07',
	selection: '#4a2c26',
	ok: '#7cc68d',
	danger: '#ff5c7c'
};

export const PLAIN: TerminalInfo = { mux: null, shell: 'C:\\Program Files\\PowerShell\\7\\pwsh.exe' };

export const SESSIONS: TerminalSession[] = [
	{ name: 'thing', windows: 3, attached: true },
	{ name: 'quiblo', windows: 1, attached: false }
];

export const WINDOWS: TerminalWindow[] = [
	{ index: 1, name: 'shell', active: true },
	{ index: 2, name: 'tests', active: false },
	{ index: 3, name: 'logs', active: false }
];

/** Let promises and the frame the output waits for go by. */
export async function settle(): Promise<void> {
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	await new Promise((done) => setTimeout(done, 40));
}
