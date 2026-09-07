// SPDX-License-Identifier: GPL-3.0-or-later
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// Exercise the installed Vite/Svelte pipeline, in a separate process so its
// NODE_ENV and compiler cache cannot be inherited from the Vitest runner.
const probe = `
import { createServer } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { resolve } from 'node:path';
const server = await createServer({
  configFile: false,
  logLevel: 'silent',
  plugins: svelte({ configFile: resolve('svelte.config.js') }),
  resolve: { alias: { $lib: resolve('src/lib') } },
  server: { middlewareMode: true, hmr: true, ws: false },
  optimizeDeps: { noDiscovery: true, include: [] }
});
try {
  const id = '/src/lib/chrome/TitleBar.svelte';
  const styleId = id + '?svelte&type=style&lang.css';
  if (process.env.NODE_ENV === 'development') {
    // Reproduce a stylesheet request arriving before the component compiles.
    // v5 caches raw .svelte source here; a later JS request cannot repair it.
    await server.transformRequest(styleId);
  }
  const js = (await server.transformRequest(id)).code;
  const style = process.env.NODE_ENV === 'production'
    ? (await server.transformRequest(styleId)).code : '';
  console.log('STYLE_PROBE:' + JSON.stringify({ js, style }));
} finally {
  await server.close();
}
`;

function compile(mode: string): { js: string; style: string } {
	const output = execFileSync(process.execPath, ['--input-type=module', '-e', probe], {
		cwd: process.cwd(),
		env: { ...process.env, NODE_ENV: mode },
		encoding: 'utf8',
		timeout: 30_000
	});
	const result = output.split('\n').find((line) => line.startsWith('STYLE_PROBE:'));
	if (!result) throw new Error(`No stylesheet probe result: ${output}`);
	return JSON.parse(result.slice('STYLE_PROBE:'.length));
}

describe('component styles survive a cold development cache', () => {
	it('carries scoped layout CSS in the component even when CSS was requested first', () => {
		const { js } = compile('development');
		expect(js).toContain('$.append_styles');
		expect(js).not.toContain('?svelte&type=style&lang.css');
		expect(js).toMatch(/\.titlebar\.svelte-/);
		expect(js).toContain('display: grid');
		expect(js).toContain('grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr)');
	}, 30_000);

	it('still extracts scoped CSS for production', () => {
		const { js, style } = compile('production');
		expect(js).not.toContain('$.append_styles');
		expect(js).toContain('?svelte&type=style&lang.css');
		expect(style).toMatch(/\.titlebar\.svelte-/);
		expect(style).toMatch(/display:\s*grid/);
		expect(style).not.toContain('<script');
	}, 30_000);
});

/**
 * Which environment the build actually thinks it is in (BUG-028, again).
 *
 * The two tests above set `NODE_ENV` themselves, so they can only ever say
 * "given this environment, the pipeline does the right thing". They passed
 * throughout the period the bug was live, because the case that was wrong is
 * the one they cannot express: **the variable not being set at all.**
 *
 * `@sveltejs/kit/vite` reads `svelte.config.js` eagerly, while `vite.config.ts`
 * is still being evaluated and before Vite has set `NODE_ENV` for the run. A
 * bare `vite dev` therefore reached the guard with nothing set, `!==
 * 'development'` answered *production*, and every development run armed the
 * exact failure the guard exists to prevent — visible in the dev server log as
 * `[postcss] … Unknown word` on a `.svelte` file.
 *
 * The probe above cannot reproduce that: it hands the plugin a `configFile`,
 * which is loaded from inside a plugin hook, by which time Vite has set the
 * variable. So the guard is checked here directly, as what it is — a decision
 * made from an environment variable at module load.
 */
describe('the environment the style decision is made from', () => {
	/** Load `svelte.config.js` fresh under one environment, as Vite would. */
	function emitCss(node_env: string | undefined): boolean {
		const env = { ...process.env };
		if (node_env === undefined) delete env.NODE_ENV;
		else env.NODE_ENV = node_env;

		const script =
			"const c = (await import('./svelte.config.js')).default;" +
			"console.log('EMIT:' + c.vitePlugin.emitCss);";
		const output = execFileSync(process.execPath, ['--input-type=module', '-e', script], {
			cwd: process.cwd(),
			env,
			encoding: 'utf8',
			timeout: 30_000
		});
		return output.includes('EMIT:true');
	}

	it('takes the safe path when nothing says which environment this is', () => {
		// The regression. An unset variable must not read as production.
		expect(emitCss(undefined)).toBe(false);
	});

	it('takes the safe path in development', () => {
		expect(emitCss('development')).toBe(false);
	});

	it('extracts CSS only when something says this is production', () => {
		expect(emitCss('production')).toBe(true);
	});

	it('is told which environment it is in by the scripts that start it', () => {
		// The belt to the guard's braces, and the part that makes the real dev
		// server deterministic rather than dependent on an inherited shell.
		const pkg = JSON.parse(readFileSync('package.json', 'utf8')) as {
			scripts: Record<string, string>;
		};

		expect(pkg.scripts.dev).toContain('NODE_ENV=development');
		expect(pkg.scripts.build).toContain('NODE_ENV=production');
	});
});
