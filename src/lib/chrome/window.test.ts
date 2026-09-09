// SPDX-License-Identifier: GPL-3.0-or-later

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';

const tauriWindow = {
	close: vi.fn(),
	minimize: vi.fn(),
	toggleMaximize: vi.fn(),
	isMaximized: vi.fn(() => Promise.resolve(true)),
	startDragging: vi.fn(),
	startResizeDragging: vi.fn(() => Promise.resolve())
};

vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => tauriWindow }));

import { appWindow, decoratesItself, shellState } from './window';

/** The three hosts, as their own webviews describe themselves. */
const AGENTS = {
	linux: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0',
	mac: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)',
	windows: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko)'
};

/** The module decides whether it is inside Tauri by looking at `window`. */
function insideTauri(yes: boolean) {
	vi.stubGlobal('window', yes ? { __TAURI_INTERNALS__: {} } : {});
}

beforeEach(() => {
	vi.clearAllMocks();
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('outside Tauri', () => {
	beforeEach(() => insideTauri(false));

	it('does nothing rather than throwing, so the UI still runs in a browser', async () => {
		await expect(appWindow.close()).resolves.toBeUndefined();
		await expect(appWindow.minimize()).resolves.toBeUndefined();
		await expect(appWindow.toggleMaximize()).resolves.toBeUndefined();
		await expect(appWindow.startDragging()).resolves.toBeUndefined();
		await expect(appWindow.startResize('North')).resolves.toBeUndefined();

		expect(tauriWindow.close).not.toHaveBeenCalled();
	});

	it('reports a window that is not maximized', async () => {
		expect(await appWindow.isMaximized()).toBe(false);
	});
});

describe('inside Tauri', () => {
	beforeEach(() => insideTauri(true));

	it('forwards each control to the platform window', async () => {
		await appWindow.close();
		await appWindow.minimize();
		await appWindow.toggleMaximize();
		await appWindow.startDragging();

		expect(tauriWindow.close).toHaveBeenCalledTimes(1);
		expect(tauriWindow.minimize).toHaveBeenCalledTimes(1);
		expect(tauriWindow.toggleMaximize).toHaveBeenCalledTimes(1);
		expect(tauriWindow.startDragging).toHaveBeenCalledTimes(1);
	});

	it('passes the edge through for a resize', async () => {
		await appWindow.startResize('SouthEast');
		expect(tauriWindow.startResizeDragging).toHaveBeenCalledWith('SouthEast');
	});

	it('reports the real maximized state', async () => {
		expect(await appWindow.isMaximized()).toBe(true);
	});
});

/**
 * BUG-029: the packaged AppImage came up ringed in blurred desktop, because
 * the window drew a card inside the corner the compositor had already drawn
 * and the 10px between them was transparent.
 */
describe('which edge the window draws', () => {
	it('draws its own card on the host that decorates nothing', () => {
		// Windows alone, now. macOS joined Linux in TASK-042: it has real
		// decorations there, so a card drawn inside a real macOS frame would be
		// the same second-card mistake BUG-029 was about, more obviously.
		expect(shellState(false, AGENTS.windows)).toBe('floating');
	});

	it('squares itself against the screen when maximized', () => {
		expect(shellState(true, AGENTS.windows)).toBe('maximized');
	});

	it.each([
		['Linux, where the compositor drew one', 'linux'],
		['macOS, where the system draws the frame', 'mac']
	] as const)('draws no card at all on %s', (_label, agent) => {
		expect(shellState(false, AGENTS[agent])).toBe('flush');
	});

	it.each(['linux', 'mac'] as const)('stays flush on %s when the window is restored', (agent) => {
		// The reason is the platform, not the size, so restoring must not hand
		// the transparent margin back.
		expect(shellState(true, AGENTS[agent])).toBe('flush');
	});

	it('reads the platform off the agent string, and only whole words', () => {
		expect(decoratesItself(AGENTS.linux)).toBe(false);
		expect(decoratesItself(AGENTS.mac)).toBe(false);
		expect(decoratesItself(AGENTS.windows)).toBe(true);
		// "Linux" inside a longer token is a product name, not a platform.
		expect(decoratesItself('Mozilla/5.0 (Windows NT 10.0) Linuxish/1.0')).toBe(true);
	});

	it('falls back to a drawn card when nothing says what the host is', () => {
		// A test environment with no navigator must not silently take the
		// Linux branch and hide the card from every other test.
		expect(shellState(false, '')).toBe('floating');
	});
});

/**
 * The macOS window policy (TASK-042).
 *
 * Three neutral glyph buttons on the right is not what a Mac window looks like,
 * and on that platform the difference between "an application" and "a web page
 * inside a custom frame" is mostly this one detail. The policy is a
 * platform-specific Tauri config, which nothing else in the suite reads — and
 * an inert config file is exactly the sort of thing that gets renamed or
 * quietly reverted with no test to notice.
 */
describe('what macOS is given instead', () => {
	const base = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
	const mac = JSON.parse(readFileSync('src-tauri/tauri.macos.conf.json', 'utf8'));

	const baseWindow = base.app.windows[0];
	const macWindow = mac.app.windows[0];

	/** The undecorated window is what Linux and Windows still get. */
	it('leaves the base configuration undecorated', () => {
		expect(baseWindow.decorations).toBe(false);
		expect(baseWindow.transparent).toBe(true);
	});

	it('gives macOS the real frame back', () => {
		expect(macWindow.decorations).toBe(true);
		// Transparency is what the drawn card needed. With a real frame it is
		// a window that lets the desktop through its own background.
		expect(macWindow.transparent).toBe(false);
	});

	/**
	 * `Overlay` is the half of the decision that keeps Spagitty's own bar: the
	 * system draws its traffic lights over the webview rather than taking a
	 * strip of it, so the workspace bar survives and only the controls change.
	 * Plain decorations would give a second title bar above the application's.
	 */
	it('keeps Spagitty own bar by overlaying the system controls on it', () => {
		expect(macWindow.titleBarStyle).toBe('Overlay');
		expect(macWindow.hiddenTitle).toBe(true);
	});

	/** A window that opened at a different size on one platform would be a
	 *  second set of numbers to keep in step. */
	it('changes nothing but the decoration', () => {
		for (const key of ['title', 'width', 'height', 'resizable', 'fullscreen'] as const) {
			expect(macWindow[key], key).toEqual(baseWindow[key]);
		}
	});
});
