// SPDX-License-Identifier: GPL-3.0-or-later

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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
	it('draws its own card on the hosts that decorate nothing', () => {
		expect(shellState(false, AGENTS.mac)).toBe('floating');
		expect(shellState(false, AGENTS.windows)).toBe('floating');
	});

	it('squares itself against the screen when maximized', () => {
		expect(shellState(true, AGENTS.mac)).toBe('maximized');
		expect(shellState(true, AGENTS.windows)).toBe('maximized');
	});

	it('draws no card at all on Linux, where the compositor drew one', () => {
		expect(shellState(false, AGENTS.linux)).toBe('flush');
	});

	it('stays flush on Linux when the window is restored', () => {
		// The reason is the compositor, not the size, so restoring must not
		// hand the transparent margin back.
		expect(shellState(true, AGENTS.linux)).toBe('flush');
	});

	it('reads the platform off the agent string, and only whole words', () => {
		expect(decoratesItself(AGENTS.linux)).toBe(false);
		expect(decoratesItself(AGENTS.mac)).toBe(true);
		// "Linux" inside a longer token is a product name, not a platform.
		expect(decoratesItself('Mozilla/5.0 (Macintosh) Linuxish/1.0')).toBe(true);
	});

	it('falls back to a drawn card when nothing says what the host is', () => {
		// A test environment with no navigator must not silently take the
		// Linux branch and hide the card from every other test.
		expect(shellState(false, '')).toBe('floating');
	});
});
