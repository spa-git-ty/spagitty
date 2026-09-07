// SPDX-License-Identifier: GPL-3.0-or-later
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/**
 * SPA mode. Tauri serves a static bundle from disk and there is no server to
 * render on, so every route falls back to index.html and routing happens
 * entirely client-side (see src/routes/+layout.ts).
 *
 * @type {import('@sveltejs/kit').Config}
 */
const config = {
	preprocess: vitePreprocess(),
	/*
	 * In dev, styles travel with the component. The v5 Vite plugin can serve
	 * raw .svelte source when a CSS request arrives before its compile cache is
	 * warm, leaving broken, unscoped styles cached for the session (BUG-028).
	 * Production still extracts CSS into independently cacheable assets.
	 *
	 * **Asked as "is this production", not "is this not development."** This
	 * file is read while Vite is still resolving its own configuration, and at
	 * that moment `NODE_ENV` is whatever the shell had — which for a plain
	 * `vite dev` is nothing at all. `!== 'development'` therefore read as
	 * *production* on every development run and armed the exact bug it was
	 * written to disarm; the postcss error was still there in the dev log.
	 *
	 * The scripts in `package.json` now say which one they are, so the answer
	 * is never a guess. This asks the question the safe way round as well: if
	 * something ever runs a build without saying so, the cost is styles inlined
	 * in the JS rather than extracted, which is a missed optimisation. The
	 * other way round the cost is a broken page.
	 */
	vitePlugin: { emitCss: process.env.NODE_ENV === 'production' },
	kit: {
		adapter: adapter({ fallback: 'index.html', strict: false }),
		alias: { $lib: 'src/lib' },
		// Only the compact, release-ready logo variants are public UI assets.
		// The source previews and fonts alongside them remain out of the bundle.
		files: { assets: 'assets/brand/favicon' }
	}
};

export default config;
