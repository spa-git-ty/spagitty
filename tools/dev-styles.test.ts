// SPDX-License-Identifier: GPL-3.0-or-later
import { execFileSync } from 'node:child_process';
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
