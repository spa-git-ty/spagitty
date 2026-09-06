// SPDX-License-Identifier: GPL-3.0-or-later
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	plugins: [sveltekit()],

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
