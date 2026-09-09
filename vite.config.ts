// SPDX-License-Identifier: GPL-3.0-or-later
import { readFileSync } from 'node:fs';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vitest/config';

const host = process.env.TAURI_DEV_HOST;

/** Where the boot script lives, and the one name the document asks for. */
const THEME_BOOT_SOURCE = 'src/theme-boot.js';
const THEME_BOOT_URL = '/theme-boot.js';

/**
 * Serve and emit `theme-boot.js` at the bundle root (BUG-031).
 *
 * The script has to be a **file** rather than an inline `<script>`, because
 * `src-tauri/tauri.conf.json` sets `default-src 'self'` and names no
 * `script-src` — so inline scripts are blocked, and the alternatives were
 * `'unsafe-inline'` for the whole application or a CSP hash that has to be kept
 * in step with an HTML file by hand. See the header of `src/theme-boot.js`.
 *
 * It cannot live in the assets directory either: `svelte.config.js` scopes that
 * to `assets/brand/favicon`, which `tools/make-brand.py --check` owns and
 * regenerates. So the file sits beside `app.html` with the rest of the shell,
 * and this plugin puts it where the document asks for it — emitted into the
 * build, and served by the dev middleware, from one source.
 */
function themeBoot(): Plugin {
	const read = () => readFileSync(THEME_BOOT_SOURCE, 'utf8');

	return {
		name: 'spagitty-theme-boot',

		// `generateBundle` rather than `writeBundle`: the file has to be part of
		// the bundle so the static adapter carries it into `build/`.
		generateBundle() {
			this.emitFile({ type: 'asset', fileName: 'theme-boot.js', source: read() });
		},

		configureServer(server) {
			server.middlewares.use((request, response, next) => {
				if (request.url?.split('?')[0] !== THEME_BOOT_URL) return next();
				response.setHeader('Content-Type', 'text/javascript');
				// Read per request, so editing it during a dev session works
				// like editing anything else.
				response.end(read());
			});
		}
	};
}

export default defineConfig({
	plugins: [sveltekit(), themeBoot()],

	// Tauri expects a fixed port and fails if it is not available.
	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
		watch: {
			// The Rust tree is rebuilt by cargo, not by vite.
			ignored: ['**/src-tauri/**', '**/crates/**', '**/target/**']
		}
	},

	/**
	 * `browser` resolution, so `import from 'svelte'` gives the client runtime
	 * rather than the server one. Components are mounted for real in the
	 * component tests, and the server runtime cannot mount.
	 */
	resolve: { conditions: ['browser'] },

	/**
	 * Tests run against happy-dom.
	 *
	 * Most of what is worth testing is logic — lane geometry, diff row pairing,
	 * store transitions, formatting — and would run in node. The components
	 * need a DOM to mount into, and they are half the frontend, so carrying one
	 * DOM implementation for the whole suite is cheaper than splitting the run
	 * in two.
	 */
	test: {
		environment: 'happy-dom',
		// See the file: Node's own `localStorage` global shadows happy-dom's.
		setupFiles: ['./vitest.setup.ts'],
		// `tools/` is here for the record check (TASK-012), which reads `agile/`
		// and `docs/` as data. It is not frontend code and is not counted for
		// coverage — see `coverage.include` below.
		include: ['src/**/*.test.ts', 'tools/**/*.test.ts'],
		coverage: {
			provider: 'v8',
			// First-party frontend code only. Amendment 10 counts nothing else,
			// in either direction.
			include: ['src/lib/**', 'src/routes/**'],
			exclude: ['src/**/*.test.ts'],
			reporter: ['text', 'json-summary'],
			// The floor. A change that drops below it fails the run rather than
			// being noticed later, or not at all.
			thresholds: { statements: 70, branches: 70, functions: 70, lines: 70 }
		}
	}
});
