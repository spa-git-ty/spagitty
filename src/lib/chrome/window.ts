// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Window controls for our own decorations.
 *
 * The window is created with `decorations: false`, so the platform draws no
 * title bar and Spagitty draws its own — the 30px bar from the design, with its
 * traffic lights. That means we also own what the platform used to provide:
 * dragging, resizing, and the close/minimize/maximize buttons.
 *
 * Everything here degrades to a no-op outside Tauri, so the UI still runs in a
 * plain browser during frontend work.
 */

import { inTauri } from '$lib/api';

/** Mirrors `ResizeDirection` in @tauri-apps/api, which is a string union. */
export type ResizeEdge =
	| 'North'
	| 'NorthEast'
	| 'East'
	| 'SouthEast'
	| 'South'
	| 'SouthWest'
	| 'West'
	| 'NorthWest';

async function currentWindow() {
	if (!inTauri()) return null;
	const { getCurrentWindow } = await import('@tauri-apps/api/window');
	return getCurrentWindow();
}

/** How the shell draws its own edge. Read by CSS as `data-window`. */
export type ShellState = 'floating' | 'maximized' | 'flush';

/**
 * Whether this host wants a window that draws its own corner and shadow.
 *
 * Everywhere but Linux, yes: the window is undecorated, so if Spagitty does not
 * draw an edge nothing does, and a hard-cornered rectangle with no shadow reads
 * as a screenshot rather than a window.
 *
 * On Linux, no — and this is BUG-029. A Linux compositor draws the window's
 * corner, its border and its shadow itself; Hyprland, KWin and Mutter all do.
 * Spagitty drawing a *second* corner inside that one leaves a 10px transparent
 * margin between the two, and a transparent margin is not empty on a compositor
 * that blurs what is behind a window: on Omarchy the packaged AppImage came up
 * ringed in a band of blurred desktop, inside the compositor's own border. The
 * card was the bug, not the blur.
 *
 * A UA string is a coarse instrument and it is the honest one available here.
 * The real question — "does this compositor decorate windows for me" — has no
 * API behind it on any desktop, and the plugin that would name the platform
 * would be a dependency added to learn something `navigator` already says.
 * WebKitGTK is the Linux webview and it says `Linux`; the other two hosts say
 * `Macintosh` and `Windows NT`.
 */
export function decoratesItself(userAgent: string): boolean {
	return !/\bLinux\b/.test(userAgent);
}

/** The `data-window` value for a host, given whether the window is maximized. */
export function shellState(maximized: boolean, userAgent?: string): ShellState {
	const agent =
		userAgent ?? (typeof navigator === 'undefined' ? '' : (navigator.userAgent ?? ''));

	// Flush wins over maximized because they ask for the same picture and the
	// Linux answer does not change when the window is restored.
	if (!decoratesItself(agent)) return 'flush';
	return maximized ? 'maximized' : 'floating';
}

export const appWindow = {
	async close(): Promise<void> {
		(await currentWindow())?.close();
	},

	async minimize(): Promise<void> {
		(await currentWindow())?.minimize();
	},

	async toggleMaximize(): Promise<void> {
		(await currentWindow())?.toggleMaximize();
	},

	async isMaximized(): Promise<boolean> {
		return (await currentWindow())?.isMaximized() ?? false;
	},

	/**
	 * Publish how the window should draw its own edge, as `data-window` on the
	 * root element (FEAT-037, BUG-029).
	 *
	 * The window draws its own corner, edge and shadow, and none of those belong
	 * on a maximized window: a floating card with a gap around it is a window
	 * that does not fit its own screen. CSS cannot ask Tauri, so the answer is
	 * put where CSS can read it.
	 *
	 * Returns an unsubscribe function, or a no-op outside Tauri.
	 */
	async watchMaximized(): Promise<() => void> {
		if (typeof document === 'undefined') return () => {};

		const apply = (maximized: boolean) => {
			document.documentElement.dataset.window = shellState(maximized);
		};

		const window_ = await currentWindow();
		if (!window_) {
			// In a plain browser there is no window to maximize, and a card with
			// a shadow is the honest thing to draw.
			apply(false);
			return () => {};
		}

		apply(await window_.isMaximized());
		return await window_.onResized(async () => apply(await window_.isMaximized()));
	},

	/** Begin a move. Used by the title bar's empty space. */
	async startDragging(): Promise<void> {
		(await currentWindow())?.startDragging();
	},

	/**
	 * Begin a resize from one edge or corner. Without decorations the
	 * compositor no longer offers resize edges, so the window supplies its own.
	 */
	async startResize(edge: ResizeEdge): Promise<void> {
		await (await currentWindow())?.startResizeDragging(edge);
	}
};
