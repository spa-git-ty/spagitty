// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The emulator: xterm.js (FEAT-130).
 *
 * With the fit add-on, which sizes it to its element; the WebGL renderer,
 * which is what keeps a busy `cargo test` from costing the window frames, and
 * which is let go for the default renderer where the machine refuses it; and
 * the web-links add-on, for addresses. Commits and files are links too, by a
 * provider of our own that asks the repository first.
 *
 * Its colours are the theme's, read when it is made and again when the theme
 * changes (FEAT-157). The multiplexer's bars and borders are given the same
 * ones, by `multiplexerStyle`.
 */

import { FitAddon } from '@xterm/addon-fit';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { WebglAddon } from '@xterm/addon-webgl';
import { Terminal, type ILink, type ITheme } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';
import { isMac } from '$lib/platform';
import type { TerminalStyle } from '$lib/types';
import type { Emulator, EmulatorHooks } from './emulator';
import { kept, pressedOf } from './keys';

/** Lines kept above the screen. */
export const SCROLLBACK = 10_000;

/**
 * A theme token as a colour xterm can read.
 *
 * A token may be a `color-mix()`, which the renderer's own parser does not
 * know. A canvas does: the token is painted and read back. `token` may also be
 * a whole colour expression, such as a `color-mix()` of two tokens.
 */
function colourOf(token: string, fallback: string): string {
	try {
		const probe = document.createElement('span');
		probe.style.color = token.startsWith('--') ? `var(${token})` : token;
		document.body.appendChild(probe);
		const computed = getComputedStyle(probe).color;
		probe.remove();
		const canvas = document.createElement('canvas');
		canvas.width = canvas.height = 1;
		const context = canvas.getContext('2d');
		if (!context || !computed) return fallback;
		context.fillStyle = computed;
		context.fillRect(0, 0, 1, 1);
		const [r, g, b] = context.getImageData(0, 0, 1, 1).data;
		return `#${[r, g, b].map((part) => part.toString(16).padStart(2, '0')).join('')}`;
	} catch {
		return fallback;
	}
}

/** The terminal's sixteen colours and its ground, from the theme's own tokens. */
function theme(): ITheme {
	const ink = colourOf('--ink', '#f3e9dd');
	const muted = colourOf('--muted', '#9a8f84');
	const red = colourOf('--danger', '#ff5c7c');
	const green = colourOf('--ok', '#7cc68d');
	const yellow = colourOf('--warn', '#f0b54a');
	const blue = colourOf('--lane-5', '#6aa7e0');
	const magenta = colourOf('--lane-4', '#b596e8');
	const cyan = colourOf('--agent', '#86e3d2');
	return {
		background: colourOf('--bg', '#1c1613'),
		foreground: ink,
		cursor: colourOf('--accent', '#f2715a'),
		cursorAccent: colourOf('--bg', '#1c1613'),
		selectionBackground: colourOf('--accent', '#f2715a') + '55',
		black: colourOf('--sunken', '#161110'),
		red,
		green,
		yellow,
		blue,
		magenta,
		cyan,
		white: ink,
		brightBlack: muted,
		brightRed: red,
		brightGreen: green,
		brightYellow: yellow,
		brightBlue: blue,
		brightMagenta: magenta,
		brightCyan: cyan,
		brightWhite: ink
	};
}

/**
 * The theme's colours for the multiplexer's status line and borders
 * (FEAT-157). The ground is the panel, a step off the terminal's own, so the
 * status line reads as a bar; the current window is the accent, faint, over
 * that ground, as a selected tab is.
 */
export function multiplexerStyle(): TerminalStyle {
	return {
		ground: colourOf('--panel', '#171514'),
		ink: colourOf('--ink', '#ebe4dc'),
		muted: colourOf('color-mix(in srgb, var(--ink) 62%, var(--panel))', '#9a948e'),
		line: colourOf('color-mix(in srgb, var(--ink) 22%, var(--bg))', '#45403d'),
		accent: colourOf('--accent', '#f2715a'),
		onAccent: colourOf('--on-accent', '#2a0e07'),
		selection: colourOf('color-mix(in srgb, var(--accent) 26%, var(--panel))', '#4a2c26'),
		ok: colourOf('--ok', '#7cc68d'),
		danger: colourOf('--danger', '#ff5c7c')
	};
}

/** Open an address the way a link in the application is opened: outside it. */
function openAddress(address: string): void {
	const link = document.createElement('a');
	link.href = address;
	link.target = '_blank';
	link.rel = 'noreferrer noopener';
	link.click();
}

export function makeXterm(hooks: EmulatorHooks): Emulator {
	const element = document.createElement('div');
	element.className = 'terminal-host';
	element.style.width = '100%';
	element.style.height = '100%';

	const style = getComputedStyle(document.documentElement);
	const terminal = new Terminal({
		scrollback: SCROLLBACK,
		fontFamily: style.getPropertyValue('--font-mono').trim() || 'monospace',
		fontSize: parseFloat(style.getPropertyValue('--fs-code')) || 14,
		cursorBlink: true,
		allowProposedApi: true,
		theme: theme()
	});
	const fit = new FitAddon();
	terminal.loadAddon(fit);
	terminal.loadAddon(new WebLinksAddon((_event, address) => openAddress(address)));

	let opened = false;
	/** xterm draws into an element that is in the document, so it is opened the first time it is fitted. */
	function open(): void {
		if (opened || !element.isConnected) return;
		opened = true;
		terminal.open(element);
		try {
			const webgl = new WebglAddon();
			// A lost context is not coming back: the default renderer takes over.
			webgl.onContextLoss(() => webgl.dispose());
			terminal.loadAddon(webgl);
		} catch {
			// No WebGL here. The default renderer is already drawing.
		}
	}

	terminal.onData((data) => hooks.onData(data));

	/**
	 * Every key is the shell's except the ones kept (`keys.ts`). A kept key is
	 * not processed here, so it reaches the window, where the layout acts on
	 * the palette and the way out; copy and paste are done on the spot.
	 */
	terminal.attachCustomKeyEventHandler((event) => {
		const what = kept(pressedOf(event), isMac());
		if (what === null) return true;
		if (event.type === 'keydown') {
			if (what === 'copy') {
				const text = terminal.getSelection();
				if (text) void navigator.clipboard.writeText(text).catch(() => {});
				event.preventDefault();
			} else if (what === 'paste') {
				void navigator.clipboard
					.readText()
					.then((text) => terminal.paste(text))
					.catch(() => {});
				event.preventDefault();
			}
		}
		return false;
	});

	terminal.registerLinkProvider({
		provideLinks(row, callback) {
			const line = terminal.buffer.active.getLine(row - 1)?.translateToString(true) ?? '';
			if (line.trim() === '') {
				callback(undefined);
				return;
			}
			hooks.links(line).then(
				(links) => {
					const found: ILink[] = links.map((link) => ({
						text: link.text,
						// Columns count from one, and the end is the last cell.
						range: { start: { x: link.start + 1, y: row }, end: { x: link.end, y: row } },
						decorations: { underline: true, pointerCursor: true },
						activate: () => hooks.open(link)
					}));
					callback(found.length > 0 ? found : undefined);
				},
				() => callback(undefined)
			);
		}
	});

	return {
		element,
		write: (bytes) => terminal.write(bytes),
		fit() {
			open();
			if (opened) {
				try {
					fit.fit();
				} catch {
					// Not laid out yet. The next resize fits it.
				}
			}
			return { cols: terminal.cols, rows: terminal.rows };
		},
		focus: () => terminal.focus(),
		selection: () => terminal.getSelection(),
		paste: (text) => terminal.paste(text),
		retheme: () => {
			terminal.options.theme = theme();
		},
		dispose: () => {
			terminal.dispose();
			element.remove();
		}
	};
}
