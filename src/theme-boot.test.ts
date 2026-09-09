// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The boot script, run as the browser runs it (BUG-031).
 *
 * `src/theme-boot.js` is the one piece of Spagitty that executes before
 * anything else exists — before SvelteKit, before `app.css` has finished, and
 * with no module system around it. It cannot be imported, so it is read off
 * disk and evaluated, which is as close to the real thing as a test can get and
 * has the useful side effect of failing if the file is renamed out from under
 * `vite.config.ts`.
 *
 * What is being checked is mostly the **refusals**. `localStorage` is not a
 * trusted input here: the realistic hostile case is not an attacker — the
 * origin is Spagitty's own bundle — it is a half-written value from a previous
 * version or a key somebody edited by hand, and the cost of getting it wrong is
 * a window that opens with no colours at all.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { readFileSync } from 'node:fs';

const SOURCE = readFileSync('src/theme-boot.js', 'utf8');
const PALETTE_KEY = 'spagitty.theme.palette';

/** Evaluate the script the way a `<script src>` tag would. */
function boot(): void {
	new Function(SOURCE)();
}

function stubStorage(initial: Record<string, string> = {}): void {
	const store = new Map(Object.entries(initial));
	vi.stubGlobal('localStorage', {
		getItem: (key: string) => store.get(key) ?? null,
		setItem: (key: string, value: string) => void store.set(key, value)
	});
}

function stubPrefersDark(dark: boolean): void {
	vi.stubGlobal('matchMedia', (query: string) => ({
		matches: dark && query.includes('dark'),
		media: query
	}));
	// The script reads `window.matchMedia`, because it runs with no bundler to
	// paper over the difference.
	window.matchMedia = globalThis.matchMedia as typeof window.matchMedia;
}

const root = () => document.documentElement;

beforeEach(() => {
	root().setAttribute('data-theme', 'light');
	root().removeAttribute('style');
	stubStorage();
	stubPrefersDark(false);
});

afterEach(() => {
	vi.unstubAllGlobals();
	root().removeAttribute('style');
});

const CACHED = JSON.stringify({
	mode: 'dark',
	tokens: { '--bg': '#1e1e2e', '--ink': '#cdd6f4', '--accent': '#eeb04d' }
});

describe('the cached palette', () => {
	it('is on the root element before anything else runs', () => {
		stubStorage({ [PALETTE_KEY]: CACHED });

		boot();

		expect(root().getAttribute('data-theme')).toBe('dark');
		expect(root().style.getPropertyValue('--bg')).toBe('#1e1e2e');
		expect(root().style.getPropertyValue('--accent')).toBe('#eeb04d');
	});

	/**
	 * The reason the whole file exists. `app.html` hard-coded `data-theme`, so
	 * a saved dark theme painted light first and corrected itself once the
	 * layout mounted — several frames on the software renderer Linux uses.
	 */
	it('overrides the document markup rather than waiting for the layout', () => {
		stubStorage({ [PALETTE_KEY]: CACHED });
		expect(root().getAttribute('data-theme')).toBe('light');

		boot();

		expect(root().getAttribute('data-theme')).toBe('dark');
	});
});

describe('what it refuses', () => {
	it.each([
		['not JSON at all', '{oh no'],
		['JSON that is not an object', '"dark"'],
		['a mode that is not a mode', JSON.stringify({ mode: 'sepia', tokens: { '--bg': '#000' } })],
		['no tokens', JSON.stringify({ mode: 'dark' })],
		['tokens that are not an object', JSON.stringify({ mode: 'dark', tokens: '--bg: #000' })]
	])('drops %s and leaves the stylesheet alone', (_label, raw) => {
		stubStorage({ [PALETTE_KEY]: raw });

		expect(() => boot()).not.toThrow();
		expect(root().style.getPropertyValue('--bg')).toBe('');
	});

	/**
	 * A property name is written straight onto `style`, so it is the one input
	 * that must not be taken on trust. Anything outside `--kebab-case` is
	 * dropped and the rest of the palette still applies — a single bad entry
	 * must not cost the whole theme.
	 */
	it('drops a token whose name is not a custom property', () => {
		stubStorage({
			[PALETTE_KEY]: JSON.stringify({
				mode: 'dark',
				tokens: { background: 'red', '--bg': '#101010' }
			})
		});

		boot();

		expect(root().style.getPropertyValue('background')).toBe('');
		expect(root().style.getPropertyValue('--bg')).toBe('#101010');
	});

	it('drops a value that is not shaped like a colour', () => {
		stubStorage({
			[PALETTE_KEY]: JSON.stringify({
				mode: 'dark',
				tokens: {
					'--bg': "url('http://elsewhere/x.png')",
					'--panel': '#181825; content: "no"',
					'--ink': 42,
					'--accent': '#eeb04d'
				}
			})
		});

		boot();

		expect(root().style.getPropertyValue('--bg')).toBe('');
		expect(root().style.getPropertyValue('--panel')).toBe('');
		expect(root().style.getPropertyValue('--ink')).toBe('');
		// The one good entry still lands.
		expect(root().style.getPropertyValue('--accent')).toBe('#eeb04d');
	});

	it('allows the derived forms a palette legitimately uses', () => {
		stubStorage({
			[PALETTE_KEY]: JSON.stringify({
				mode: 'light',
				tokens: {
					'--muted': 'rgba(76, 79, 105, 0.72)',
					'--surface': 'color-mix(in srgb, #e6e9ef 34%, #fff)'
				}
			})
		});

		boot();

		expect(root().style.getPropertyValue('--muted')).toBe('rgba(76, 79, 105, 0.72)');
		expect(root().style.getPropertyValue('--surface')).toContain('color-mix');
	});

	it('survives storage being unreadable', () => {
		vi.stubGlobal('localStorage', {
			getItem: () => {
				throw new Error('denied');
			}
		});

		expect(() => boot()).not.toThrow();
	});
});

describe('with nothing cached', () => {
	/**
	 * A fresh install on a dark desktop used to open white and stay white until
	 * the layout mounted. The stylesheet already carries both of the default
	 * family's palettes keyed off this attribute, so setting it is the whole
	 * fix — no colours are needed here at all.
	 */
	it('takes the desktop preference', () => {
		stubPrefersDark(true);

		boot();

		expect(root().getAttribute('data-theme')).toBe('dark');
		// Deliberately nothing: `app.css` has the values.
		expect(root().style.getPropertyValue('--bg')).toBe('');
	});

	it('leaves the markup alone on a light desktop', () => {
		stubPrefersDark(false);

		boot();

		expect(root().getAttribute('data-theme')).toBe('light');
	});

	it('leaves the markup alone where the preference cannot be read', () => {
		vi.stubGlobal('matchMedia', undefined);
		// @ts-expect-error — deleting it is the case being tested.
		delete window.matchMedia;

		expect(() => boot()).not.toThrow();
		expect(root().getAttribute('data-theme')).toBe('light');
	});
});

/**
 * The joins nothing else reads together.
 *
 * This script is wired up in three separate files and is inert if any one of
 * them stops agreeing — and inert means "the theme flashes again", which is a
 * defect nobody reports because it looks like the application being slow.
 * Asserted here rather than in `tools/`, beside the file they are about.
 */
describe('how it reaches the page', () => {
	const html = readFileSync('src/app.html', 'utf8');
	const vite = readFileSync('vite.config.ts', 'utf8');
	const tauri = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));

	it('is asked for by the document, before SvelteKit', () => {
		expect(html).toContain('<script src="%sveltekit.assets%/theme-boot.js"></script>');
		// Order matters: after `%sveltekit.head%` it would run after the
		// application's own scripts have been parsed, which is the flash.
		expect(html.indexOf('theme-boot.js')).toBeLessThan(html.indexOf('%sveltekit.head%'));
	});

	it('is emitted into the bundle under that name', () => {
		expect(vite).toContain("fileName: 'theme-boot.js'");
		expect(vite).toContain("'src/theme-boot.js'");
	});

	/**
	 * The reason it is a file. `default-src 'self'` with no `script-src` means
	 * scripts come from `'self'` only, so an inline script is blocked — and if
	 * a later change adds `'unsafe-inline'` to get around something, this
	 * script's whole justification is gone and somebody should have to notice.
	 */
	it('needs no relaxation of the content security policy', () => {
		const csp: string = tauri.app.security.csp;
		expect(csp).toContain("default-src 'self'");
		expect(csp).not.toContain('unsafe-inline\'; script-src');
		expect(csp.match(/script-src[^;]*/)?.[0] ?? '').not.toContain('unsafe-inline');
	});
});
