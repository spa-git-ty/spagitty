// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A file's kind as a small coloured tag (FEAT-149).
 */

import { describe, expect, it } from 'vitest';
import { flushSync, render } from '../../testing/mount';
import FileGlyph from './FileGlyph.svelte';
import { PLAIN, fileKind } from './kind';

describe('fileKind', () => {
	it('reads the extension, whatever its case and however deep the path', () => {
		expect(fileKind('src/lib/files/kind.ts').tag).toBe('TS');
		expect(fileKind('crates/core/src/LIB.RS').tag).toBe('RS');
		expect(fileKind('src/routes/+page.svelte').tag).toBe('SV');
		expect(fileKind('package.json').tag).toBe('{}');
	});

	it('knows the files known by their whole name', () => {
		expect(fileKind('Dockerfile').label).toBe('Docker');
		expect(fileKind('LICENSE').label).toBe('Licence');
		expect(fileKind('LICENSE-MIT').label).toBe('Licence');
		expect(fileKind('Cargo.lock').label).toBe('Lock file');
		expect(fileKind('.gitignore').label).toBe('Git file');
		expect(fileKind('.env.local').label).toBe('Environment');
	});

	it('gives what it does not know a plain grey sheet', () => {
		expect(fileKind('Brewfile')).toBe(PLAIN);
		expect(fileKind('notes.weird')).toBe(PLAIN);
		expect(fileKind('.hidden')).toBe(PLAIN);
		expect(PLAIN.hue).toBeNull();
	});
});

describe('FileGlyph', () => {
	it('draws the tag without writing it into the row’s text', () => {
		const view = render(FileGlyph, { path: 'a/b.toml' });
		flushSync();

		const tag = view.get('.tag');
		expect(tag.dataset.tag).toBe('TML');
		expect(tag.classList.contains('long')).toBe(true);
		expect(tag.textContent).toBe('');
		view.destroy();
	});

	it('draws a folder as a folder, open or shut', () => {
		const view = render(FileGlyph, { path: 'src', folder: true, open: true });
		expect(view.get('svg.folder').classList.contains('open')).toBe(true);
		view.destroy();
	});
});
