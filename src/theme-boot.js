// SPDX-License-Identifier: GPL-3.0-or-later

/*
 * The first frame's colours, painted before SvelteKit exists (BUG-031).
 *
 * `src/app.html` opened with `data-theme="light"` and `theme.init()` ran in the
 * layout's `onMount`. Between those two moments the window is up, `app.css` has
 * been parsed, and the application paints Catppuccin Latte — so somebody whose
 * saved theme is Mocha, or Dracula, or an imported dark palette, watched their
 * Git client open white and then change its mind. On the software renderer the
 * Linux build deliberately uses (see `src-tauri/src/platform.rs`) that gap is
 * not a frame, it is several.
 *
 * This runs in the document head, synchronously, before anything paints. It
 * reads the palette the previous session cached and puts it on the root
 * element, which is exactly what `theme.svelte.ts` will do again a moment later
 * with the same values — so the handover is invisible rather than a second
 * change of mind.
 *
 * # Why a file rather than an inline script
 *
 * `src-tauri/tauri.conf.json` sets `default-src 'self'` and names no
 * `script-src`, so scripts fall back to `'self'` and **an inline script is
 * blocked**. The alternatives were to add `'unsafe-inline'` — which would
 * disarm the policy for the whole application to save one HTTP request against
 * the local bundle — or to hash this script into the CSP, which means a hash in
 * a JSON file that has to be kept in step with a script in an HTML file, with
 * silent breakage as the failure mode. A same-origin file needs no policy
 * change at all. `vite.config.ts` emits it to the bundle root; it is served in
 * development by the same plugin.
 *
 * # Why the cache rather than the palette table
 *
 * `themes.ts` is 550 lines of data and the boot path must not wait on it. What
 * is needed is not "the Gruvbox dark palette", it is "whatever was on screen
 * last time" — so `theme.svelte.ts` writes the *resolved* custom properties out
 * on every change and this reads them back. It also means a palette that came
 * from somewhere other than the built-in table is cached on the same terms,
 * which is what FEAT-080 needs.
 *
 * # Everything here is validated
 *
 * `localStorage` is not a trusted input. It is per-origin and this origin is
 * Spagitty's own bundle, so the realistic case is not an attacker — it is a
 * half-written value from a previous version, or a key somebody edited by hand.
 * A malformed entry must leave the stylesheet's own boot values alone rather
 * than write nonsense into `style`, so every name and every value is checked
 * against a pattern before it is set, and anything that fails is dropped
 * silently. A theme is not worth a broken window.
 */
(function () {
	var PALETTE_KEY = 'spagitty.theme.palette';

	/** `--lane-1`, `--fs-ui`. Nothing else may be written to the root. */
	var NAME = /^--[a-z0-9-]+$/;

	/*
	 * Colours and the handful of non-colour values a palette carries.
	 *
	 * Deliberately a whitelist of shapes rather than a blacklist of characters:
	 * `(` and `)` have to be allowed for `rgb()`, `color-mix()` and the rest,
	 * and a rule that allowed those while forbidding `url(` would be a filter
	 * to get wrong. This allows hex, the functional colour notations, plain
	 * keywords, and nothing containing a quote, a semicolon or a backslash.
	 */
	var VALUE = /^[a-zA-Z0-9%#(),.\-+/* ]{1,200}$/;

	function apply() {
		var raw;
		try {
			raw = localStorage.getItem(PALETTE_KEY);
		} catch (error) {
			// Private mode, or a webview with storage disabled. The stylesheet's
			// own values are already correct for the default family.
			return;
		}
		if (!raw) return fallBackToSystem();

		var cached;
		try {
			cached = JSON.parse(raw);
		} catch (error) {
			return fallBackToSystem();
		}

		if (!cached || (cached.mode !== 'light' && cached.mode !== 'dark')) {
			return fallBackToSystem();
		}
		if (!cached.tokens || typeof cached.tokens !== 'object') {
			return fallBackToSystem();
		}

		var root = document.documentElement;
		root.setAttribute('data-theme', cached.mode);

		for (var name in cached.tokens) {
			if (!Object.prototype.hasOwnProperty.call(cached.tokens, name)) continue;
			var value = cached.tokens[name];
			if (typeof value !== 'string') continue;
			if (!NAME.test(name) || !VALUE.test(value)) continue;
			root.style.setProperty(name, value);
		}
	}

	/*
	 * No usable cache: a fresh install, or a cleared store.
	 *
	 * The markup used to hard-code `light`, so a first run on a dark desktop
	 * opened white and stayed white until `init()` sampled the preference. The
	 * stylesheet already carries both of the default family's palettes, keyed
	 * off this attribute, so setting it is the whole of the fix — no colours
	 * are needed here at all.
	 */
	function fallBackToSystem() {
		try {
			if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
				document.documentElement.setAttribute('data-theme', 'dark');
			}
		} catch (error) {
			// Leave the markup's own default.
		}
	}

	apply();
})();
