// SPDX-License-Identifier: GPL-3.0-or-later

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PALETTE_KEY, theme } from './theme.svelte';
import { DEFAULT_FAMILY, paletteOf, properties } from './themes';

const MODE_KEY = 'spagitty.theme';
const FAMILY_KEY = 'spagitty.theme.family';
const SOURCE_KEY = 'spagitty.theme.source';

function stubStorage(initial: Record<string, string> = {}) {
	const store = new Map(Object.entries(initial));
	vi.stubGlobal('localStorage', {
		getItem: (k: string) => store.get(k) ?? null,
		setItem: (k: string, v: string) => void store.set(k, v),
		removeItem: (k: string) => void store.delete(k)
	});
	return store;
}

function stubPrefersDark(dark: boolean) {
	vi.stubGlobal('matchMedia', (query: string) => ({
		matches: dark && query.includes('dark'),
		media: query
	}));
}

/** What the root element is actually carrying. */
function property(name: string): string {
	return document.documentElement.style.getPropertyValue(name);
}

beforeEach(() => {
	stubStorage();
	stubPrefersDark(false);
	theme.setFamily(DEFAULT_FAMILY);
	theme.setMode('light');
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('applying', () => {
	it('writes the mode onto the root element, which is what the stylesheet keys off', () => {
		theme.setMode('dark');
		expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
		expect(theme.mode).toBe('dark');
		expect(theme.isDark).toBe(true);

		theme.setMode('light');
		expect(document.documentElement.getAttribute('data-theme')).toBe('light');
		expect(theme.isDark).toBe(false);
	});

	it('writes every token of the chosen palette', () => {
		// A token left unset would keep whatever the previous theme put there,
		// which is how half a theme ends up on screen.
		theme.setFamily('gruvbox');
		theme.setMode('dark');

		for (const [name, value] of Object.entries(properties(paletteOf('gruvbox', 'dark')))) {
			expect(property(name), name).toBe(value);
		}
	});

	it('replaces every token when the family changes', () => {
		theme.setFamily('dracula');
		theme.setFamily('tokyo-night');

		expect(property('--bg')).toBe(paletteOf('tokyo-night', 'light').bg);
		expect(property('--lane-1')).toBe(paletteOf('tokyo-night', 'light').lanes[0]);
	});

	it('toggles the mode and leaves the family alone', () => {
		theme.setFamily('gruvbox');

		theme.toggle();
		expect(theme.mode).toBe('dark');
		expect(theme.family).toBe('gruvbox');

		theme.toggle();
		expect(theme.mode).toBe('light');
	});

	it('names what is on, family and mode, because both repaint the lane canvas', () => {
		// The canvas resolves its colours from the stylesheet and repaints when
		// this changes. A boolean could not say the family moved.
		theme.setFamily('dracula');
		theme.setMode('dark');
		expect(theme.id).toBe('dracula-dark');

		theme.setFamily('gruvbox');
		expect(theme.id).toBe('gruvbox-dark');
	});

	it('says what the family calls the variant, not just light or dark', () => {
		theme.setFamily('catppuccin');
		theme.setMode('dark');
		expect(theme.variant.name).toBe('Mocha');

		theme.setMode('light');
		expect(theme.variant.name).toBe('Latte');
	});

	it('persists both halves of the choice', () => {
		const store = stubStorage();

		theme.setFamily('tokyo-night');
		theme.setMode('dark');

		expect(store.get(FAMILY_KEY)).toBe('tokyo-night');
		expect(store.get(MODE_KEY)).toBe('dark');
	});
});

describe('init', () => {
	it('restores a stored family and mode', () => {
		stubStorage({ [FAMILY_KEY]: 'gruvbox', [MODE_KEY]: 'dark' });
		stubPrefersDark(false);

		theme.init();

		expect(theme.family).toBe('gruvbox');
		expect(theme.mode).toBe('dark');
	});

	it('still honours a mode stored before families existed', () => {
		// The key has not changed, so an existing install keeps its light or
		// dark and gains the default family rather than losing its setting.
		stubStorage({ [MODE_KEY]: 'dark' });
		stubPrefersDark(false);

		theme.init();

		expect(theme.mode).toBe('dark');
		expect(theme.family).toBe(DEFAULT_FAMILY);
	});

	it('prefers the stored mode over the OS preference', () => {
		stubStorage({ [MODE_KEY]: 'light' });
		stubPrefersDark(true);

		theme.init();

		expect(theme.mode).toBe('light');
	});

	it('falls back to the OS preference when nothing is stored', () => {
		stubStorage();
		stubPrefersDark(true);

		theme.init();

		expect(theme.mode).toBe('dark');
		expect(theme.family).toBe(DEFAULT_FAMILY);
	});

	it('ignores a stored mode that is not a mode', () => {
		stubStorage({ [MODE_KEY]: 'solarized' });
		stubPrefersDark(true);

		theme.init();

		expect(theme.mode).toBe('dark');
	});

	it('ignores a stored family that is not a family', () => {
		// The file invites hand-editing, and a typo must not leave the window
		// with no colours at all.
		stubStorage({ [FAMILY_KEY]: 'monokai', [MODE_KEY]: 'light' });

		theme.init();

		expect(theme.family).toBe(DEFAULT_FAMILY);
		expect(property('--bg')).toBe(paletteOf(DEFAULT_FAMILY, 'light').bg);
	});

	it('survives storage being unreadable', () => {
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new Error('denied');
			},
			setItem: () => {
				throw new Error('denied');
			}
		});
		stubPrefersDark(true);

		expect(() => theme.init()).not.toThrow();
		expect(theme.mode).toBe('dark');
		expect(theme.family).toBe(DEFAULT_FAMILY);
	});

	it('defaults to light where the OS preference cannot be read', () => {
		stubStorage();
		vi.stubGlobal('matchMedia', undefined);

		theme.init();

		expect(theme.mode).toBe('light');
	});
});

/**
 * A media query list that can actually change its mind, which the plain stub
 * above cannot: it has no listener API at all, and the store is written to
 * survive that (an old WebKitGTK is a real target). These tests need the other
 * case.
 */
function stubLiveQuery(dark: boolean) {
	const listeners = new Set<() => void>();
	const query = {
		matches: dark,
		media: '(prefers-color-scheme: dark)',
		addEventListener: (_: string, listener: () => void) => void listeners.add(listener),
		removeEventListener: (_: string, listener: () => void) => void listeners.delete(listener)
	};
	vi.stubGlobal('matchMedia', () => query);

	return {
		get listeners() {
			return listeners.size;
		},
		/** What the desktop just did. */
		change(nowDark: boolean) {
			query.matches = nowDark;
			for (const listener of [...listeners]) listener();
		}
	};
}

/**
 * Where light-or-dark comes from, which used to be collapsed into what it
 * currently is (BUG-031).
 */
describe('the source of the mode', () => {
	it('follows the desktop while it is set to, and keeps following', () => {
		stubStorage();
		const system = stubLiveQuery(false);

		theme.followSystem();
		expect(theme.source).toBe('system');
		expect(theme.mode).toBe('light');

		// The whole defect: this used to change nothing, because the OS was
		// sampled once at startup and the answer written down as a decision.
		system.change(true);
		expect(theme.mode).toBe('dark');
		expect(document.documentElement.getAttribute('data-theme')).toBe('dark');

		system.change(false);
		expect(theme.mode).toBe('light');
	});

	it('keeps the family while the desktop changes the mode', () => {
		stubStorage();
		const system = stubLiveQuery(false);

		theme.setFamily('nord');
		theme.followSystem();
		system.change(true);

		expect(theme.family).toBe('nord');
		expect(property('--bg')).toBe(paletteOf('nord', 'dark').bg);
	});

	/** Picking a mode is the act of taking over. A control that changed the
	 *  mode and left the source alone would be overruled by the next system
	 *  change, which is a control that does not work. */
	it('stops following as soon as a mode is chosen', () => {
		stubStorage();
		const system = stubLiveQuery(false);

		theme.followSystem();
		theme.setMode('dark');
		expect(theme.source).toBe('manual');

		system.change(false);
		expect(theme.mode).toBe('dark');
	});

	it('drops the listener when it stops following, and on dispose', () => {
		stubStorage();
		const system = stubLiveQuery(false);

		theme.followSystem();
		expect(system.listeners).toBe(1);

		theme.setMode('light');
		expect(system.listeners).toBe(0);

		theme.followSystem();
		theme.dispose();
		expect(system.listeners).toBe(0);
	});

	it('does not add a second listener when told to follow twice', () => {
		stubStorage();
		const system = stubLiveQuery(false);

		theme.followSystem();
		theme.followSystem();

		expect(system.listeners).toBe(1);
	});

	it('remembers the source across a restart', () => {
		const store = stubStorage();
		stubLiveQuery(true);

		theme.followSystem();
		expect(store.get(SOURCE_KEY)).toBe('system');

		theme.init();
		expect(theme.source).toBe('system');
		expect(theme.mode).toBe('dark');
	});

	/**
	 * The migration. An install from before the source existed has a mode and
	 * no source, and that mode might have been chosen — so it is treated as a
	 * choice. Demoting somebody's explicit dark theme to "whatever the desktop
	 * says" would lose a real preference; the reverse mistake costs one visit
	 * to Appearance.
	 */
	it('treats a mode stored before the source existed as a decision', () => {
		stubStorage({ [MODE_KEY]: 'dark' });
		stubLiveQuery(false);

		theme.init();

		expect(theme.source).toBe('manual');
		expect(theme.mode).toBe('dark');
	});

	/** A fresh install follows the desktop rather than freezing its first
	 *  answer, which is what `init()` used to do. */
	it('starts a fresh install following the desktop', () => {
		stubStorage();
		const system = stubLiveQuery(true);

		theme.init();

		expect(theme.source).toBe('system');
		expect(theme.mode).toBe('dark');

		system.change(false);
		expect(theme.mode).toBe('light');
	});

	it('ignores a stored source that is not a source', () => {
		stubStorage({ [SOURCE_KEY]: 'omarchy', [MODE_KEY]: 'dark' });
		stubLiveQuery(false);

		theme.init();

		expect(theme.source).toBe('manual');
	});
});

/**
 * The cache the boot script reads (BUG-031).
 *
 * Nothing in `theme.svelte.ts` reads it back, so nothing but a test can notice
 * it going stale — and a stale entry is a window that paints last week's theme
 * for a frame and then corrects itself, which is the exact defect this was
 * added to remove.
 */
describe('the cached palette', () => {
	it('is written on every change, resolved rather than named', () => {
		const store = stubStorage();

		theme.setFamily('dracula');
		theme.setMode('dark');

		const cached = JSON.parse(store.get(PALETTE_KEY) as string);
		expect(cached.mode).toBe('dark');
		expect(cached.tokens).toEqual(properties(paletteOf('dracula', 'dark')));
	});

	it('follows a mode the desktop chose, not only one the user did', () => {
		const store = stubStorage();
		const system = stubLiveQuery(false);

		theme.followSystem();
		system.change(true);

		expect(JSON.parse(store.get(PALETTE_KEY) as string).mode).toBe('dark');
	});

	it('carries every token the boot script would need', () => {
		const store = stubStorage();
		theme.setFamily('everforest');

		const cached = JSON.parse(store.get(PALETTE_KEY) as string);
		// The names are the ones `app.css` declares, so the boot script can set
		// them without knowing anything about families.
		for (const name of ['--bg', '--panel', '--ink', '--accent', '--lane-1', '--lane-5']) {
			expect(cached.tokens[name], name).toBeTruthy();
		}
	});
});
