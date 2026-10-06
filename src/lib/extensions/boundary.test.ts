// SPDX-License-Identifier: GPL-3.0-or-later

import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { reviewRecord } from '../../testing/extension-fixtures';
import type { ReviewRecord } from './types';

/** Every non-test source file in the extension UI. */
function sources(): { name: string; text: string }[] {
	const here = join(process.cwd(), 'src/lib/extensions');
	return readdirSync(here)
		.filter((name) => !name.endsWith('.test.ts'))
		.map((name) => ({ name, text: readFileSync(join(here, name), 'utf8') }));
}

describe('the promises the extension UI makes', () => {
	it('calls the backend from exactly one file', () => {
		const calling = sources().filter(({ text }) => /\binvoke\s*\(/.test(text) || text.includes('@tauri-apps/api/core'));
		expect(calling.map((s) => s.name)).toEqual(['api.ts']);
	});

	it('never hands extension content to the browser as markup', () => {
		for (const { name, text } of sources()) {
			expect(text, name).not.toMatch(/\{@html/);
			expect(text, name).not.toMatch(/innerHTML|outerHTML|insertAdjacentHTML/);
		}
	});

	it('makes no network request and holds no token', () => {
		for (const { name, text } of sources()) {
			expect(text, name).not.toMatch(/\bfetch\s*\(/);
			expect(text, name).not.toContain('XMLHttpRequest');
			expect(text, name).not.toContain('WebSocket');
			expect(text, name).not.toMatch(/\bBearer\b|\bAuthorization\b/);
		}
	});

	it('evaluates nothing an extension wrote', () => {
		for (const { name, text } of sources()) {
			expect(text, name).not.toMatch(/\beval\s*\(|new Function\s*\(/);
			expect(text, name).not.toMatch(/import\s*\(\s*[^'"`]/);
		}
	});

	it('reads the review record exactly as the host writes it', () => {
		const record: ReviewRecord = reviewRecord();
		expect(record.result.findings[0].severity).toBe('high');
		expect(record.result.findings[1].path).toBeUndefined();
		expect(record.gate?.gate).toBe('blocked');
		expect(record.snapshot.contentDigest).toMatch(/^sha256:[0-9a-f]{64}$/);
	});
});
