// SPDX-License-Identifier: GPL-3.0-or-later

import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { TARGETS, parseManifest } from '../../packages/extension-sdk/src/manifest';
import { attachRequest, BUN_TARGETS, create, dataDir, hostTarget, main, validatePath } from './cli';

let scratch = '';
afterEach(() => {
	if (scratch) rmSync(scratch, { recursive: true, force: true });
	scratch = '';
});

describe('ext create', () => {
	it('scaffolds an extension whose manifest the host accepts, with nothing left to fill in', () => {
		scratch = mkdtempSync(join(tmpdir(), 'ext-create-'));
		const dir = create('com.example.greeter', join(scratch, 'greeter'), 'Tests');
		const manifest = parseManifest(readFileSync(join(dir, 'extension.json'), 'utf8'));
		expect(manifest.id).toBe('com.example.greeter');
		expect(manifest.name).toBe('Greeter');
		expect(manifest.publisher).toBe('Tests');
		for (const file of ['src/main.ts', 'test/extension.test.ts', 'README.md', 'package.json']) {
			expect(readFileSync(join(dir, file), 'utf8'), file).not.toMatch(/\{\{\w+\}\}/);
		}
	});

	it('refuses an id that is not reverse-DNS and a folder that is not empty', () => {
		scratch = mkdtempSync(join(tmpdir(), 'ext-create-'));
		expect(() => create('Greeter', join(scratch, 'x'))).toThrow(/reverse-DNS/);
		create('com.example.one', join(scratch, 'one'));
		expect(() => create('com.example.two', join(scratch, 'one'))).toThrow(/not empty/);
	});
});

describe('ext dev', () => {
	it('leaves a request where Spagitty looks, naming the folder', () => {
		scratch = mkdtempSync(join(tmpdir(), 'ext-dev-'));
		const file = attachRequest(join(process.cwd(), 'examples/extensions/hello'), { SPAGITTY_DATA_DIR: scratch });
		expect(file).toBe(join(scratch, 'extensions', 'development-requests', 'com.example.hello.json'));
		expect(JSON.parse(readFileSync(file, 'utf8')).path).toBe(join(process.cwd(), 'examples/extensions/hello'));
	});

	it('finds the data directory Tauri uses on each platform', () => {
		expect(dataDir({ APPDATA: 'C:\\Users\\a\\AppData\\Roaming' }, 'win32')).toBe(join('C:\\Users\\a\\AppData\\Roaming', 'dev.spagitty.app'));
		expect(dataDir({ XDG_DATA_HOME: '/x' }, 'linux')).toBe(join('/x', 'dev.spagitty.app'));
		expect(dataDir({}, 'darwin')).toMatch(/Library[\\/]Application Support[\\/]dev\.spagitty\.app$/);
		expect(dataDir({ SPAGITTY_DATA_DIR: '/override' }, 'linux')).toBe('/override');
	});
});

describe('ext validate and build', () => {
	it('validates the example as the host would', () => {
		const problems = validatePath(join(process.cwd(), 'examples/extensions/hello'));
		expect(problems.filter((p) => !p.startsWith('warning:'))).toEqual([]);
	});

	it('knows how to build every target bun can, and names this machine', () => {
		expect(TARGETS).toContain(hostTarget());
		expect(Object.keys(BUN_TARGETS).every((t) => (TARGETS as readonly string[]).includes(t))).toBe(true);
		expect(BUN_TARGETS['aarch64-pc-windows-msvc']).toBeUndefined();
	});

	it('answers usage with a non-zero status for an unknown action', async () => {
		expect(await main(['frobnicate'])).toBe(1);
		expect(await main(['validate', join(process.cwd(), 'examples/extensions/hello')])).toBe(0);
		expect(existsSync(join(process.cwd(), 'tools/extensions/cli.ts'))).toBe(true);
	});
});
