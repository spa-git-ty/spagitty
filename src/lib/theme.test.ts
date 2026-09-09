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
		// `omarchy` was the invalid value here until FEAT-080 made it a real
		// one, which is the sort of thing this assertion exists to notice.
		stubStorage({ [SOURCE_KEY]: 'whatever-the-desktop-says', [MODE_KEY]: 'dark' });
		stubLiveQuery(false);

		theme.init();

		expect(theme.source).toBe('manual');
	});

	/** The third source restores like the other two. */
	it('remembers that the desktop palette was chosen', () => {
		const store = stubStorage();
		stubLiveQuery(true);

		theme.followDesktop();
		expect(store.get(SOURCE_KEY)).toBe('omarchy');

		theme.init();
		expect(theme.source).toBe('omarchy');
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

/**
 * Following the desktop's whole palette (FEAT-080).
 *
 * The store's side of it: what a reading does, what a *failed* reading does,
 * and what makes the revision move. The derivation itself is `omarchy.test.ts`.
 */
describe('following the desktop palette', () => {
	const SUSHI = {
		available: true,
		name: 'sushi-dark-palette',
		layout: 'state' as const,
		reason: null,
		palette: {
			mode: 'dark',
			background: '#191724',
			foreground: '#fcfcfd',
			accent: '#cf6348',
			darkBg: '#13101e',
			lighterBg: '#2a2735',
			muted: '#5f5e63',
			selectionBackground: '#cf6348',
			selectionForeground: '#191724',
			red: '#bda8a4',
			green: '#8fa487',
			yellow: '#cd9071',
			blue: '#778291',
			magenta: '#a1758a',
			cyan: '#4a89a4',
			ansi: Array(16).fill(null)
		}
	};

	it('is not offered until a palette has actually been read', () => {
		stubStorage();
		expect(theme.desktopAvailable).toBe(false);

		theme.receiveDesktop(SUSHI);
		expect(theme.desktopAvailable).toBe(true);
		expect(theme.desktopName).toBe('sushi-dark-palette');
	});

	it('paints the desktop palette once it is being followed', () => {
		stubStorage();
		theme.receiveDesktop(SUSHI);
		theme.followDesktop();

		expect(theme.source).toBe('omarchy');
		expect(property('--bg')).toBe('#191724');
		expect(property('--accent')).toBe('#cf6348');
		// Derived, not copied: the desktop's own muted is 2.75:1 on that ground.
		expect(property('--muted')).not.toBe('#5f5e63');
	});

	/** The desktop decides light or dark too, or `app.css`'s boot values fight
	 *  the inline properties on any token a palette does not set. */
	it('takes the mode from the desktop palette', () => {
		stubStorage();
		theme.setMode('light');
		theme.receiveDesktop(SUSHI);
		theme.followDesktop();
		theme.receiveDesktop(SUSHI);

		expect(theme.mode).toBe('dark');
		expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
	});

	/**
	 * `omarchy-theme-set` replaces a directory, so there is a moment with no
	 * readable palette at all. Falling back to the built-in family for that
	 * moment would be a flash of Catppuccin in the middle of a theme change.
	 */
	it('keeps the last good palette through an unreadable reading', () => {
		stubStorage();
		theme.receiveDesktop(SUSHI);
		theme.followDesktop();

		theme.receiveDesktop({
			available: false,
			name: null,
			layout: null,
			palette: null,
			reason: 'mid-swap'
		});

		expect(property('--bg')).toBe('#191724');
		expect(theme.desktopReason).toBe('mid-swap');
	});

	/** A family and a desktop palette are two answers to the same question. */
	it('stops following when a family is chosen', () => {
		stubStorage();
		theme.receiveDesktop(SUSHI);
		theme.followDesktop();

		theme.setFamily('nord');

		expect(theme.source).toBe('manual');
		expect(property('--bg')).toBe(paletteOf('nord', 'dark').bg);
	});

	/** Choosing a family is not an opt-out of following light/dark. */
	it('leaves the system source alone when a family is chosen', () => {
		stubStorage();
		stubLiveQuery(false);
		theme.followSystem();

		theme.setFamily('nord');

		expect(theme.source).toBe('system');
	});

	it('says the desktop name rather than a variant it is not', () => {
		stubStorage();
		theme.receiveDesktop(SUSHI);
		theme.followDesktop();

		expect(theme.label).toBe('sushi-dark-palette');

		theme.setFamily('catppuccin');
		expect(theme.label).toBe('Mocha');
	});
});

/**
 * The revision, which is what a colour cache should read (FEAT-080).
 *
 * `id` is `family-mode`. Every followed palette is `omarchy-dark`, so a switch
 * between two dark desktop themes is invisible to it — and `LaneCanvas`
 * invalidates its lane colours and its whole portrait cache from exactly that
 * string.
 */
describe('the palette revision', () => {
	const desktopTheme = (background: string, accent: string) => ({
		available: true,
		name: 'a-theme',
		layout: 'state' as const,
		reason: null,
		palette: {
			mode: 'dark',
			background,
			foreground: '#fcfcfd',
			accent,
			darkBg: null,
			lighterBg: null,
			muted: null,
			selectionBackground: null,
			selectionForeground: null,
			red: null,
			green: null,
			yellow: null,
			blue: null,
			magenta: null,
			cyan: null,
			ansi: Array(16).fill(null)
		}
	});

	it('moves when the family changes', () => {
		stubStorage();
		theme.setFamily('catppuccin');
		const before = theme.revision;

		theme.setFamily('nord');

		expect(theme.revision).toBeGreaterThan(before);
	});

	/** The case `id` cannot see: same source, same mode, different colours. */
	it('moves between two desktop palettes that share an id', () => {
		stubStorage();
		theme.receiveDesktop(desktopTheme('#191724', '#cf6348'));
		theme.followDesktop();
		const id = theme.id;
		const before = theme.revision;

		theme.receiveDesktop(desktopTheme('#101018', '#6ba9c5'));

		expect(theme.id, 'the identity genuinely does not change').toBe(id);
		expect(theme.revision).toBeGreaterThan(before);
	});

	/** An event that re-reads an identical palette is not a repaint. */
	it('stands still when nothing actually changed', () => {
		stubStorage();
		theme.receiveDesktop(desktopTheme('#191724', '#cf6348'));
		theme.followDesktop();
		const before = theme.revision;

		theme.receiveDesktop(desktopTheme('#191724', '#cf6348'));

		expect(theme.revision).toBe(before);
	});
});
