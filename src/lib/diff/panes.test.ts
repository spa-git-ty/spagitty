// SPDX-License-Identifier: GPL-3.0-or-later

import { readFileSync } from 'node:fs';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, press, render } from '../../testing/mount';
import type { DiffView } from './store.svelte';
import type { FileChange, FileDiff, Hunk } from '$lib/types';

import DiffPane from './DiffPane.svelte';
import FileList from './FileList.svelte';

/**
 * Both components take what they render (FEAT-034), so these mount them with
 * props rather than driving a store first. What a store does with a selection
 * is `store.test.ts`; what the Stash screen does with these same two
 * components is `stash/panes.test.ts`.
 */

function change(path: string, overrides: Partial<FileChange> = {}): FileChange {
	return {
		path,
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 2,
		removed: 1,
		...overrides
	};
}

const oneHunk: Hunk = {
	oldStart: 1,
	oldLines: 3,
	newStart: 1,
	newLines: 3,
	header: '@@ -1,3 +1,3 @@',
	lines: [
		{ origin: 'context', old: 1, new: 1, text: 'keep me' },
		{ origin: 'removed', old: 2, new: null, text: 'gone' },
		{ origin: 'added', old: null, new: 2, text: 'arrived' },
		{ origin: 'context', old: 3, new: 3, text: 'keep me too' }
	]
};

function file(path: string, overrides: Partial<FileDiff> = {}): FileDiff {
	return {
		path,
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 1,
		removed: 1,
		hunks: [oneHunk],
		...overrides
	};
}

/** The list's props, with the parts a test does not care about filled in. */
function list(overrides: Record<string, unknown> = {}) {
	return {
		files: [] as FileChange[],
		selected: null as string | null,
		onselect: (_path: string) => {},
		...overrides
	};
}

/** The pane's props, likewise. */
function pane(overrides: Record<string, unknown> = {}) {
	return {
		file: null as FileDiff | null,
		path: null as string | null,
		error: null as string | null,
		loading: false,
		view: 'unified' as DiffView,
		focus: 0,
		...overrides
	};
}

beforeEach(() => {
	vi.clearAllMocks();
	// Nothing in these tests scrolls for real.
	Element.prototype.scrollIntoView = () => {};
});

describe('FileList', () => {
	it('lists the files it is given with their line counts', () => {
		const view = render(
			FileList,
			list({ files: [change('a.txt'), change('b/c.txt', { added: 5 })] })
		);

		expect(view.all('.file')).toHaveLength(2);
		expect(view.text()).toContain('a.txt');
		expect(view.text()).toContain('+2');
		expect(view.text()).toContain('+5');

		view.destroy();
	});

	it('says why a file has no counts rather than showing +0 −0', () => {
		const view = render(
			FileList,
			list({
				files: [
					change('logo.bin', { binary: true, added: 0, removed: 0 }),
					change('huge.sql', { tooLarge: true, added: 0, removed: 0 })
				]
			})
		);

		expect(view.text()).toContain('bin');
		expect(view.text()).toContain('big');
		expect(view.text()).not.toContain('+0');

		view.destroy();
	});

	it('reports the path when a row is clicked', () => {
		const onselect = vi.fn();
		const view = render(FileList, list({ files: [change('a.txt'), change('b.txt')], onselect }));

		click(view.all('.file')[1]);

		expect(onselect).toHaveBeenCalledWith('b.txt');
		view.destroy();
	});

	it('marks the selected row and only that one', () => {
		const view = render(
			FileList,
			list({ files: [change('a.txt'), change('b.txt')], selected: 'b.txt' })
		);

		const rows = view.all('.file');
		expect(rows[0].classList.contains('selected')).toBe(false);
		expect(rows[1].classList.contains('selected')).toBe(true);

		view.destroy();
	});

	it('walks the list with the arrow keys, and reaches its ends', () => {
		// Keyboard movement through the files, which is what the Diff screen's
		// Prev/Next buttons do — the same `step` on either store.
		const onstep = vi.fn();
		const files = [change('a.txt'), change('b.txt'), change('c.txt')];
		const view = render(FileList, list({ files, selected: 'a.txt', onstep }));

		press(view.get('.files'), 'ArrowDown');
		expect(onstep).toHaveBeenLastCalledWith(1);

		press(view.get('.files'), 'ArrowUp');
		expect(onstep).toHaveBeenLastCalledWith(-1);

		// Home and End are a step long enough to reach either end whatever the
		// list holds; clamping is the store's job and is tested there.
		press(view.get('.files'), 'End');
		expect(onstep).toHaveBeenLastCalledWith(3);

		press(view.get('.files'), 'Home');
		expect(onstep).toHaveBeenLastCalledWith(-3);

		view.destroy();
	});

	it('leaves the arrow keys alone when it was given no way to step', () => {
		// The list is then click-only, and the key belongs to whatever else on
		// screen wants it.
		const view = render(FileList, list({ files: [change('a.txt')], selected: 'a.txt' }));

		const event = press(view.get('.files'), 'ArrowDown');
		expect(event.defaultPrevented).toBe(false);

		view.destroy();
	});

	it('keeps a dotfile reading as a dotfile', () => {
		// The column elides its head, which needs `direction: rtl`; without the
		// left-to-right mark the leading dot is reordered to the end.
		const view = render(FileList, list({ files: [change('.gitignore')] }));

		expect(view.get('.path').textContent).toBe('‎.gitignore');
		view.destroy();
	});

	it('says so when there are no files, in the words it was given', () => {
		const plain = render(FileList, list());
		expect(plain.text()).toContain('No file changes');
		plain.destroy();

		const stash = render(FileList, list({ empty: 'This entry changed nothing.' }));
		expect(stash.text()).toContain('This entry changed nothing.');
		stash.destroy();
	});
});

describe('DiffPane', () => {
	it('renders a unified hunk with numbers on both sides', () => {
		const view = render(DiffPane, pane({ file: file('a.txt'), path: 'a.txt' }));

		expect(view.text()).toContain('@@ -1,3 +1,3 @@');
		const removed = view.get('.line.removed');
		const added = view.get('.line.added');
		expect(removed.textContent).toContain('gone');
		expect(added.textContent).toContain('arrived');
		// A removed line has an old number and no new one.
		expect(removed.querySelectorAll('.num')[0].textContent).toBe('2');
		expect(removed.querySelectorAll('.num')[1].textContent).toBe('');

		view.destroy();
	});

	it('renders the same lines side by side in split view', () => {
		const view = render(DiffPane, pane({ file: file('a.txt'), path: 'a.txt', view: 'split' }));

		expect(view.text()).toContain('before');
		expect(view.text()).toContain('after');
		const pairs = view.all('.pair');
		expect(pairs).toHaveLength(3); // context, the changed pair, context
		expect(pairs[1].textContent).toContain('gone');
		expect(pairs[1].textContent).toContain('arrived');

		view.destroy();
	});

	it('tints the empty half of an uneven pairing', () => {
		const uneven = file('a.txt', {
			hunks: [
				{
					...oneHunk,
					lines: [
						{ origin: 'removed', old: 1, new: null, text: 'one' },
						{ origin: 'removed', old: 2, new: null, text: 'two' },
						{ origin: 'added', old: null, new: 1, text: 'merged' }
					]
				}
			]
		});

		const view = render(DiffPane, pane({ file: uneven, path: 'a.txt', view: 'split' }));

		expect(view.all('.side.blank')).toHaveLength(1);
		view.destroy();
	});

	it('says a binary file has no lines', () => {
		const view = render(
			DiffPane,
			pane({
				file: file('logo.bin', { binary: true, hunks: [] }),
				path: 'logo.bin'
			})
		);

		expect(view.text()).toContain('Binary file');
		view.destroy();
	});

	it('says an over-large file is too large', () => {
		const view = render(
			DiffPane,
			pane({
				file: file('huge.sql', { tooLarge: true, hunks: [] }),
				path: 'huge.sql'
			})
		);

		expect(view.text()).toContain('too large');
		view.destroy();
	});

	it('distinguishes a mode-only change from an empty pane', () => {
		const view = render(
			DiffPane,
			pane({ file: file('script.sh', { hunks: [] }), path: 'script.sh' })
		);

		expect(view.text()).toContain("only the file's mode changed");
		view.destroy();
	});

	it('shows a file error instead of the hunks', () => {
		const view = render(DiffPane, pane({ path: 'a.txt', error: 'no file a.txt in that commit' }));

		expect(view.text()).toContain('no file a.txt in that commit');
		view.destroy();
	});

	it('asks for a file to be selected when none is', () => {
		const view = render(DiffPane, pane());
		expect(view.text()).toContain('Select a file');
		view.destroy();
	});

	it('paints the @@ header on the same surface as the lines, so the left rule continues through it', () => {
		const source = readFileSync('src/lib/diff/DiffPane.svelte', 'utf8');
		const style = source.slice(source.indexOf('<style>'));
		const head = style.match(/\.hunk-head \{[^}]+\}/)?.[0] ?? '';
		expect(head).toMatch(/background:\s*var\(--bg\)/);
		expect(head).not.toMatch(/var\(--panel\)/);
	});

	it('brings the focused hunk into view', () => {
		const scrolled: HTMLElement[] = [];
		Element.prototype.scrollIntoView = function () {
			scrolled.push(this as HTMLElement);
		};

		const two = file('a.txt', {
			hunks: [oneHunk, { ...oneHunk, header: '@@ -20,3 +20,3 @@' }]
		});
		const view = render(DiffPane, pane({ file: two, path: 'a.txt', focus: 1 }));
		flushSync();

		expect(scrolled.at(-1)?.textContent).toContain('@@ -20,3 +20,3 @@');
		view.destroy();
	});
});
