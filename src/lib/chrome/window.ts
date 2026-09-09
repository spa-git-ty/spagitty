// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Window controls, where Spagitty provides them.
 *
 * On Linux and Windows the window is created with `decorations: false`, so the
 * platform draws no title bar and Spagitty draws its own — the 30px bar from
 * the design, with its own controls. That means it also owns what the platform
 * used to provide: dragging, resizing, and close/minimize/maximize.
 *
 * **macOS is now the exception** (TASK-042). Three evenly weighted neutral
 * glyph buttons on the *right* is not what a Mac window looks like, and the
 * gap between "an application" and "a web page in a custom frame" on that
 * platform is mostly this. `src-tauri/tauri.macos.conf.json` turns the real
 * decorations back on with an overlay title bar, so macOS draws its own traffic
 * lights at the top left, over Spagitty's bar, and Spagitty leaves room for
 * them and draws none of its own. Everything below still works there — the
 * commands are the same — it is simply that nothing calls most of them.
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
 * **Windows only, now** (TASK-042). The question is really "did the platform
 * already draw an edge", and there are now two reasons the answer can be yes.
 *
 * On Linux the compositor draws the corner, the border and the shadow itself;
 * Hyprland, KWin and Mutter all do. Spagitty drawing a *second* corner inside
 * that one leaves a 10px transparent margin between the two, and a transparent
 * margin is not empty on a compositor that blurs what is behind a window: on
 * Omarchy the packaged AppImage came up ringed in a band of blurred desktop,
 * inside the compositor's own border (BUG-029). The card was the bug.
 *
 * On macOS the window is now genuinely decorated —
 * `src-tauri/tauri.macos.conf.json` sets `decorations: true` with an overlay
 * title bar, so the system supplies the frame, the corner, the shadow and the
 * traffic lights. The same second-card argument applies, and a card drawn
 * inside a real macOS frame would be the more obviously wrong of the two.
 *
 * Windows keeps `decorations: false` and keeps drawing its own, because that is
 * where an undecorated window is still what ships.
 *
 * Written as "not one of the two platforms that decorate" rather than "is
 * Windows", deliberately: an unrecognised host — a plain browser during
 * frontend work, or a webview whose agent says something new — gets the card,
 * which is the honest picture for something that may have no frame at all. The
 * other way round, a new host would come up as a hard-cornered rectangle with
 * no shadow and look like a screenshot.
 *
 * A UA string is a coarse instrument and it is the honest one available here.
 * The real question has no API behind it on any desktop, and the plugin that
 * would name the platform would be a dependency added to learn something
 * `navigator` already says. WebKitGTK says `Linux`, macOS says `Macintosh`, and
 * Windows says `Windows NT`.
 */
export function decoratesItself(userAgent: string): boolean {
	return !/\bLinux\b/.test(userAgent) && !/\bMacintosh\b/.test(userAgent);
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
