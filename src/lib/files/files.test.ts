// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Files screen, mounted (FEAT-131): the file, its blame and peek, the
 * keys, the tree.
 */

import { readFileSync } from 'node:fs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import type { BlameCommit, CompactBlame, FilesEntry, FilesRead } from '$lib/types';

const { goto, want } = vi.hoisted(() => ({ goto: vi.fn(), want: vi.fn() }));
vi.mock('$app/navigation', () => ({ goto }));

/** The Graph, as far as Files asks anything of it: show this commit. */
vi.mock('$lib/graph/store.svelte', () => ({ graph: { want } }));

vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => false),
	filesResolve: vi.fn(),
	filesTree: vi.fn(),
	filesRead: vi.fn(),
	filesFind: vi.fn(),
	filesSpans: vi.fn(),
	filesBlame: vi.fn(),
	filesBefore: vi.fn()
}));

import * as api from '$lib/api';
import FileTree from './FileTree.svelte';
import FileView from './FileView.svelte';
import ImageDiff from '$lib/diff/ImageDiff.svelte';
import { files } from './store.svelte';

const resolve = vi.mocked(api.filesResolve);
const tree = vi.mocked(api.filesTree);
const read = vi.mocked(api.filesRead);
const findFiles = vi.mocked(api.filesFind);
const spans = vi.mocked(api.filesSpans);
const blame = vi.mocked(api.filesBlame);
const before = vi.mocked(api.filesBefore);

const MAIN = 'a'.repeat(40);
const BODY = 'use std::fs;\n\nfn one() {\n    two();\n}\n\nfn two() {}\n';

function entry(path: string, kind: FilesEntry['kind'] = 'file'): FilesEntry {
	return { path, name: path.slice(path.lastIndexOf('/') + 1), kind };
}

function aFile(path: string, overrides: Partial<FilesRead> = {}): FilesRead {
	return {
		path,
		revision: MAIN,
		blob: `blob-${path}`,
		size: BODY.length,
		grammar: 'rust',
		colour: true,
		content: { kind: 'text', text: BODY },
		...overrides
	};
}

function aCommit(overrides: Partial<BlameCommit> = {}): BlameCommit {
	return {
		id: 'c'.repeat(40),
		short: 'ccccccc',
		summary: 'Add the two functions',
		authorName: 'Ada Lovelace',
		authorEmail: 'ada@example.com',
		time: 1_700_000_000,
		agent: null,
		path: 'src/lib.rs',
		previous: null,
		previousPath: null,
		...overrides
	};
}

const PERSON = aCommit();
const AGENT = aCommit({
	id: 'e'.repeat(40),
	short: 'eeeeeee',
	summary: 'Call two from one (#214)',
	authorName: 'Mahmoud',
	authorEmail: 'mahmoud@example.com',
	time: 1_760_000_000,
	agent: 'Codex',
	previous: 'c'.repeat(40),
	previousPath: 'src/lib.rs'
});
/** Lines 3 to 5 are the agent's; the rest are Ada's. Two blocks of hers, one of its. */
const BLAME: CompactBlame = {
	path: 'src/lib.rs',
	revision: MAIN,
	commits: [PERSON, AGENT],
	lines: [0, 0, 1, 1, 1, 0, 0],
	origin: [1, 2, 3, 4, 5, 6, 7],
	refused: null
};

async function settle(): Promise<void> {
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	flushSync();
}

let view: Mounted | null = null;

/** Open a file and mount the view on it, with the blame in. */
async function opened(path = 'src/lib.rs', onreveal = vi.fn()) {
	await files.enter('/repos/thing', 'main');
	await files.openFile(path);
	await settle();
	view = render(FileView, { onreveal });
	await settle();
	return { view, onreveal };
}

const line = (n: number) => view!.get(`.line[data-line="${n}"]`);
const code = (n: number) => line(n).querySelector('.text') as HTMLElement;
const status = () => view!.get('.status').textContent?.replace(/\s+/g, ' ').trim() ?? '';
const keys = () => view!.get('.pane .viewport');

beforeEach(() => {
	vi.clearAllMocks();
	localStorage.clear();
	files.clearState();
	vi.stubGlobal('requestAnimationFrame', (work: FrameRequestCallback) => setTimeout(() => work(0), 0));

	resolve.mockResolvedValue({ id: MAIN, short: MAIN.slice(0, 7) });
	tree.mockImplementation(async (_rev, dir) =>
		dir === '' ? [entry('src', 'folder'), entry('vendor', 'submodule'), entry('README.md')] : [entry(`${dir}/lib.rs`)]
	);
	read.mockImplementation(async (_rev, path) => aFile(path));
	spans.mockResolvedValue([[1, 3, 0, 9]]);
	blame.mockResolvedValue(BLAME);
	before.mockResolvedValue(null);
	findFiles.mockResolvedValue({ paths: [], more: false });
});

afterEach(() => {
	view?.destroy();
	view = null;
	vi.unstubAllGlobals();
});

describe('a file', () => {
	it('draws every line with its number, and colours what has its colour', async () => {
		await opened();

		expect(view!.all('.line')).toHaveLength(7);
		expect(view!.all('.number').map((number) => number.textContent)).toEqual(['1', '2', '3', '4', '5', '6', '7']);
		expect(code(3).textContent).toBe('fn one() {');
		// Line 1's runs came back: `use` is a keyword.
		expect(code(1).querySelector('.tok-keyword')?.textContent).toBe('use');
		expect(spans).toHaveBeenCalledWith('blob-src/lib.rs', 'rust', 0, expect.any(Number));
	});

	/** The item: `VIEW · revision · file · language · line:col · %`. */
	it('says where you are in the status line', async () => {
		await opened();

		expect(status()).toMatch(/^VIEW main lib\.rs/);
		expect(status()).toContain('rust · tree-sitter');
		expect(status()).toMatch(/1:1 14%$/);
		expect(view!.get('.status .mode').getAttribute('title')).toContain('never writes');
	});

	it('does not claim tree-sitter for a language it has no grammar for', async () => {
		read.mockResolvedValue(aFile('Main.kt', { grammar: null, content: { kind: 'text', text: 'fun main() {}\n' } }));
		await opened('Main.kt');

		expect(status()).toContain('kotlin');
		expect(status()).not.toContain('tree-sitter');
		expect(code(1).querySelector('.tok-keyword')?.textContent).toBe('fun');
	});

	/**
	 * View-only is structural: there is nothing in the file to type into, and
	 * nothing that offers to edit or to open an editor.
	 */
	it('has nothing to type into and nothing that edits', async () => {
		await opened();

		const pane = view!.get('.pane');
		expect(pane.querySelector('input, textarea, [contenteditable]')).toBeNull();
		const said = view!.target.textContent ?? '';
		for (const word of ['Edit', 'Save', 'Open in editor']) expect(said).not.toContain(word);

		const was = code(3).textContent;
		for (const key of ['i', 'a', 'o', 'x', 'd', 'c', 'r', 'p', ':']) press(keys(), key);
		expect(code(3).textContent).toBe(was);
		expect(view!.all('.line')).toHaveLength(7);
	});

	it('names each open file in a tab and marks the one shown', async () => {
		await opened();
		await files.openFile('README.md');
		await settle();

		const tabs = view!.all('[role="tab"]');
		expect(tabs.map((tab) => tab.textContent?.trim())).toEqual(['lib.rs', 'README.md']);
		expect(tabs.map((tab) => tab.getAttribute('aria-selected'))).toEqual(['false', 'true']);

		click(tabs[0]);
		await settle();
		expect(files.file?.path).toBe('src/lib.rs');

		click(view!.get('button[aria-label="Close README.md"]'));
		await settle();
		expect(view!.all('[role="tab"]')).toHaveLength(1);
	});

	it('says which revision a tab was opened at when it is not the one shown', async () => {
		resolve.mockImplementation(async (rev) => ({ id: rev === 'side' ? 'b'.repeat(40) : MAIN, short: 'x' }));
		await opened();
		await files.openFile('README.md', { rev: 'side' });
		await settle();

		const tabs = view!.all('[role="tab"]');
		expect(tabs[0].textContent?.replace(/\s+/g, ' ').trim()).toBe('lib.rs main');
		expect(tabs[1].textContent?.trim()).toBe('README.md');
	});

	it('opens the tree at a folder when its breadcrumb segment is pressed', async () => {
		read.mockImplementation(async (_rev, path) => aFile(path));
		const { onreveal } = await opened('crates/core/src/lib.rs');

		const crumbs = view!.all('.crumbs button');
		expect(crumbs.map((crumb) => crumb.textContent)).toEqual(['crates', 'core', 'src']);
		expect(view!.get('.crumbs .leaf').textContent).toBe('lib.rs');

		click(crumbs[1]);
		expect(onreveal).toHaveBeenCalledWith('crates/core');
	});

	it('says to choose a file when none is open', async () => {
		await files.enter('/repos/thing', 'main');
		view = render(FileView, {});

		expect(view.text()).toContain('Choose a file from the tree');
		expect(status()).toBe('VIEW main');
	});
});

describe('files that are not text', () => {
	const kinds: [string, FilesRead['content'], number, RegExp][] = [
		['a binary file says binary and its size', { kind: 'binary' }, 4_404_019, /binary · 4\.2 MB/],
		[
			'a Git LFS pointer is shown as a pointer',
			{ kind: 'lfs', oid: 'sha256:4d7a21', objectSize: 12_345_678 },
			132,
			/Git LFS pointer · stands for 11\.8 MB.*sha256:4d7a21/
		],
		['a file over the limit is not opened, and its size is said', { kind: 'tooLarge' }, 62 * 1024 * 1024, /62\.0 MB · too large to open here/],
		['a path the revision does not have says so', { kind: 'missing' }, 0, /gone\.rs is not in main/]
	];

	for (const [name, content, size, said] of kinds) {
		it(name, async () => {
			read.mockResolvedValue(aFile('gone.rs', { content, size, grammar: null, colour: false }));
			await opened('gone.rs');

			expect(view!.get('.pane').textContent?.replace(/\s+/g, ' ')).toMatch(said);
			expect(view!.all('.line')).toHaveLength(0);
			expect(blame).not.toHaveBeenCalled();
		});
	}

	it('an image is drawn by the image viewer, alone', async () => {
		read.mockResolvedValue(
			aFile('logo.png', { content: { kind: 'image', mime: 'image/png', base64: 'AAAA' }, size: 68, grammar: null })
		);
		await opened('logo.png');

		const picture = view!.get('.picture img');
		expect(picture.getAttribute('src')).toBe('data:image/png;base64,AAAA');
		// One picture: nothing to compare it with, so no modes.
		expect(view!.all('.picture img')).toHaveLength(1);
		expect(view!.find('.mode-tabs')).toBeNull();
		expect(view!.get('.picture').textContent).toContain('68 B');
	});

	it('the image viewer still compares two pictures everywhere else', () => {
		const diff = render(ImageDiff, {
			diff: { path: 'a.png', isImage: true, mime: 'image/png', oldSize: 1, newSize: 2, oldBase64: 'AA', newBase64: 'BB' }
		});

		expect(diff.find('.mode-tabs')).not.toBeNull();
		expect(diff.all('img')).toHaveLength(2);
		diff.destroy();
	});
});

describe('blame', () => {
	it('heads each block once, with who, why and how long ago', async () => {
		await opened();

		const heads = view!.all('.blame .who').map((who) => who.textContent);
		expect(heads).toEqual(['Ada Lovelace', 'Codex', 'Ada Lovelace']);
		expect(line(1).querySelector('.why')?.textContent).toBe('Add the two functions');
		expect(line(1).querySelector('.age')?.textContent).toMatch(/\d+ (mo|y)/);
		// The second line of a block has the bar and no head.
		expect(line(2).querySelector('.who')).toBeNull();
	});

	it('gives every line an age bar', async () => {
		await opened();

		expect(view!.all('.blame .bar')).toHaveLength(7);
		const strength = (n: number) => Number((line(n).querySelector('.bar') as HTMLElement).style.opacity);
		// The agent's commit is the newer of the two.
		expect(strength(3)).toBeGreaterThan(strength(1));
		expect(strength(1)).toBeGreaterThan(0);
	});

	/** Agent identity is unchanged from 2.0: the agent's colour and the hexagon. */
	it('marks an agent’s lines with the agent’s name and hexagon, and a person’s with a portrait', async () => {
		await opened();

		expect(line(3).querySelector('.blame')?.classList.contains('agent')).toBe(true);
		expect(line(4).querySelector('.blame')?.classList.contains('agent')).toBe(true);
		expect(line(3).querySelector('.hex')).not.toBeNull();
		expect(line(3).querySelector('.avatar')).toBeNull();

		expect(line(1).querySelector('.blame')?.classList.contains('agent')).toBe(false);
		expect(line(1).querySelector('.avatar')).not.toBeNull();
	});

	it('lights every line a commit wrote when its block is pressed', async () => {
		await opened();

		click(line(1).querySelector('.blame') as HTMLElement);
		expect(view!.all('.line.lit').map((row) => row.dataset.line)).toEqual(['1', '2', '6', '7']);

		click(line(4).querySelector('.blame') as HTMLElement);
		expect(view!.all('.line.lit').map((row) => row.dataset.line)).toEqual(['3', '4', '5']);

		click(line(4).querySelector('.blame') as HTMLElement);
		expect(view!.all('.line.lit')).toHaveLength(0);
	});

	it('goes with b, and comes back', async () => {
		await opened();

		press(keys(), 'b');
		expect(view!.all('.blame')).toHaveLength(0);
		expect(view!.all('.line')).toHaveLength(7);

		press(keys(), 'b');
		expect(view!.all('.blame')).toHaveLength(7);
	});

	/**
	 * With the blame hidden its column is still in the grid, at no width. A
	 * number and text left to flow took the first two columns and were drawn
	 * over each other, the text cut to the number's width.
	 */
	it('keeps the number and the text in their own columns, with the blame or without it', () => {
		const source = readFileSync('src/lib/files/FileView.svelte', 'utf8');
		const rule = (selector: string) =>
			new RegExp(`\\n\\t${selector.replace('.', '\\.')} \\{[^}]*grid-column: (\\d)`).exec(source)?.[1];
		expect(rule('.number')).toBe('2');
		expect(rule('.text')).toBe('3');
	});

	it('is widened by dragging its edge, which goes when it does', async () => {
		await opened();
		const edge = () => view!.all('[role="separator"][aria-label="Resize the blame"]');

		expect(edge()).toHaveLength(1);
		press(keys(), 'b');
		expect(edge()).toHaveLength(0);
		press(keys(), 'b');
		expect(edge()).toHaveLength(1);
	});

	it('is drawn after the text: the lines are there before the blame is', async () => {
		let answer: (value: CompactBlame) => void = () => {};
		blame.mockImplementation(() => new Promise((done) => (answer = done)));
		await opened();

		expect(view!.all('.line')).toHaveLength(7);
		expect(view!.all('.blame .who')).toHaveLength(0);

		answer(BLAME);
		await settle();
		expect(view!.all('.blame .who')).toHaveLength(3);
	});
});

describe('the peek', () => {
	it('opens under the line that was pressed, in the flow of the file', async () => {
		before.mockResolvedValue('    three();');
		await opened();

		click(code(4));
		await settle();

		const peek = view!.get('.peek');
		// The row after line 4's row, and before line 5's: inline, not over the code.
		const rows = view!.all('.viewport > .row');
		const at = rows.findIndex((row) => row.contains(peek));
		expect(rows[at - 1].contains(line(4))).toBe(true);
		expect(rows[at + 1].contains(line(5))).toBe(true);
		expect(peek.closest('[popover], dialog')).toBeNull();
	});

	it('says who, when, the pull request, why, and what the line was before', async () => {
		before.mockResolvedValue('    three();');
		await opened();

		click(code(4));
		await settle();

		const peek = view!.get('.peek');
		const said = peek.textContent?.replace(/\s+/g, ' ').trim() ?? '';
		expect(said).toContain('Codex for Mahmoud');
		expect(said).toContain('#214');
		expect(said).toContain('eeeeeee');
		expect(said).toContain('Call two from one (#214)');
		expect(peek.classList.contains('agent')).toBe(true);
		// Struck through, without the indentation it had.
		expect(peek.querySelector('.was s')?.textContent).toBe('three();');
		expect(view!.all('.peek button').map((button) => button.textContent?.trim())).toEqual([
			'Commit',
			'Blame before this',
			'Investigate line'
		]);
	});

	it('has no “was” for a line the commit only added', async () => {
		await opened();

		click(code(1));
		await settle();

		const peek = view!.get('.peek');
		expect(peek.querySelector('.was')).toBeNull();
		expect(peek.classList.contains('agent')).toBe(false);
		expect(peek.textContent).toContain('Ada Lovelace');
		expect(peek.textContent).not.toContain('for Ada');
	});

	it('Commit shows it on the Graph', async () => {
		await opened();
		click(code(4));
		await settle();

		click(view!.all('.peek button')[0]);

		expect(want).toHaveBeenCalledWith('e'.repeat(40));
		expect(goto).toHaveBeenCalledWith('/');
	});

	it('Blame before this reads the file at the commit before', async () => {
		await opened();
		click(code(4));
		await settle();
		read.mockClear();

		click(view!.all('.peek button')[1]);
		await settle();

		expect(read).toHaveBeenCalledWith('c'.repeat(40), 'src/lib.rs');
		expect(files.rev).toBe('c'.repeat(40));
	});

	it('cannot blame before the commit that added the file, and says why', async () => {
		await opened();
		click(code(1));
		await settle();

		const button = view!.all('.peek button')[1] as HTMLButtonElement;
		expect(button.disabled).toBe(true);
		expect(button.getAttribute('title')).toBe('ccccccc added this file');
	});

	it('Investigate line goes to the line’s life, at the revision read', async () => {
		await opened();
		click(code(4));
		await settle();

		click(view!.all('.peek button')[2]);

		const went = new URL(`http://x${goto.mock.calls.at(-1)?.[0]}`);
		expect(went.pathname).toBe('/investigate');
		expect(Object.fromEntries(went.searchParams)).toEqual({
			kind: 'line',
			path: 'src/lib.rs',
			line: '4',
			rev: MAIN
		});
	});

	it('closes on the same line again, and on Escape', async () => {
		await opened();

		click(code(4));
		await settle();
		click(code(4));
		expect(view!.find('.peek')).toBeNull();

		press(keys(), 'Enter');
		await settle();
		expect(view!.find('.peek')).not.toBeNull();
		press(keys(), 'Escape');
		expect(view!.find('.peek')).toBeNull();
	});

	it('opens from a click only while Blame on click is on (FEAT-153)', async () => {
		await opened();
		const toggle = view!.get('.head .toggle');
		expect(toggle.getAttribute('aria-pressed')).toBe('true');

		click(code(4));
		await settle();
		expect(view!.find('.peek')).not.toBeNull();

		// Off: the open peek closes, and a click only moves the cursor.
		click(toggle);
		expect(toggle.getAttribute('aria-pressed')).toBe('false');
		expect(view!.find('.peek')).toBeNull();
		click(code(2));
		await settle();
		expect(view!.find('.peek')).toBeNull();
		expect(files.cursor.line).toBe(1);

		// Enter still asks.
		press(keys(), 'Enter');
		await settle();
		expect(view!.find('.peek')).not.toBeNull();
	});

	it('remembers Blame on click for the repository', async () => {
		await opened();
		click(view!.get('.head .toggle'));
		view!.destroy();
		view = null;

		files.clearState();
		await opened();
		expect(files.blameOnClick).toBe(false);
		expect(view!.get('.head .toggle').getAttribute('aria-pressed')).toBe('false');
	});

	it('says why when there is no blame to show', async () => {
		blame.mockResolvedValue({ ...BLAME, commits: [], lines: [], origin: [], refused: 'tooLarge' });
		await opened();

		click(code(2));
		await settle();

		expect(view!.get('.peek').textContent).toContain('too large to blame');
	});
});

describe('the mouse', () => {
	it('puts the cursor on a line number, and selects a range with shift', async () => {
		await opened();

		click(line(2).querySelector('.number') as HTMLElement);
		expect(line(2).classList.contains('cursor')).toBe(true);
		expect(view!.find('.peek')).toBeNull();

		click(line(5).querySelector('.number') as HTMLElement, { shiftKey: true });
		expect(view!.all('.line.selected').map((row) => row.dataset.line)).toEqual(['2', '3', '4', '5']);
		expect(status()).toMatch(/5:1/);
	});
});

describe('the keys', () => {
	it('move the cursor, and the status line follows', async () => {
		await opened();

		press(keys(), 'j');
		press(keys(), 'j');
		expect(line(3).classList.contains('cursor')).toBe(true);
		expect(status()).toMatch(/3:1 43%$/);

		press(keys(), 'k');
		expect(line(2).classList.contains('cursor')).toBe(true);

		press(keys(), 'G');
		expect(line(7).classList.contains('cursor')).toBe(true);
		expect(status()).toMatch(/7:1 100%$/);

		press(keys(), 'g');
		press(keys(), 'g');
		expect(line(1).classList.contains('cursor')).toBe(true);
	});

	it('take half a screen with Ctrl+d and Ctrl+u', async () => {
		await opened();

		const down = press(keys(), 'd', { ctrlKey: true });
		expect(down.defaultPrevented).toBe(true);
		expect(files.cursor.line).toBeGreaterThan(0);

		press(keys(), 'u', { ctrlKey: true });
		expect(files.cursor.line).toBe(0);
	});

	it('step through the blame blocks with ]b and [b', async () => {
		await opened();

		press(keys(), ']');
		press(keys(), 'b');
		expect(line(3).classList.contains('cursor')).toBe(true);
		// The b of `]b` did not turn the blame off.
		expect(view!.all('.blame')).toHaveLength(7);

		press(keys(), ']');
		press(keys(), 'b');
		expect(line(6).classList.contains('cursor')).toBe(true);

		press(keys(), '[');
		press(keys(), 'b');
		expect(line(3).classList.contains('cursor')).toBe(true);
	});

	it('search with /, mark what was found, and go on with n and N', async () => {
		await opened();

		press(keys(), '/');
		await settle();
		const field = view!.get('.status .search input') as HTMLInputElement;
		field.value = 'two';
		fire(field, 'input');
		press(field, 'Enter');
		await settle();

		expect(view!.find('.status .search')).toBeNull();
		expect(line(4).classList.contains('cursor')).toBe(true);
		expect(view!.get('.hit').textContent).toBe('two');
		expect(status()).toMatch(/4:5/);

		press(keys(), 'n');
		expect(line(7).classList.contains('cursor')).toBe(true);
		press(keys(), 'N');
		expect(line(4).classList.contains('cursor')).toBe(true);
	});

	it('search backwards with ?', async () => {
		await opened();

		press(keys(), '?');
		await settle();
		expect(view!.get('.status .search').textContent).toContain('?');
		const field = view!.get('.status .search input') as HTMLInputElement;
		field.value = 'fn';
		fire(field, 'input');
		press(field, 'Enter');
		await settle();

		// Backwards from the first line wraps to the last match.
		expect(line(7).classList.contains('cursor')).toBe(true);
	});

	it('say when a word is nowhere, and leave the search on Escape', async () => {
		await opened();

		press(keys(), '/');
		await settle();
		const field = view!.get('.status .search input') as HTMLInputElement;
		field.value = 'nowhere';
		fire(field, 'input');
		press(field, 'Enter');
		await settle();
		expect(status()).toContain('not found: nowhere');

		press(keys(), '/');
		await settle();
		press(view!.get('.status .search input'), 'Escape');
		await settle();
		expect(view!.find('.status .search')).toBeNull();
	});

	it('copy path:line@sha with y', async () => {
		const writeText = vi.fn(() => Promise.resolve());
		vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText } });
		await opened();

		press(keys(), 'j');
		press(keys(), 'y');
		await settle();

		expect(writeText).toHaveBeenCalledWith('src/lib.rs:2@aaaaaaa');
	});

	it('leave the application its own shortcuts', async () => {
		await opened();

		expect(press(keys(), 'f', { ctrlKey: true }).defaultPrevented).toBe(false);
		expect(press(keys(), 'p', { ctrlKey: true }).defaultPrevented).toBe(false);
		expect(press(keys(), 'j').defaultPrevented).toBe(true);
	});
});

describe('the tree', () => {
	async function aTree() {
		await files.enter('/repos/thing', 'main');
		await settle();
		view = render(FileTree, {});
		await settle();
		return view;
	}

	const rows = () => view!.all('.rows .row');
	const names = () => rows().map((row) => row.textContent?.replace(/[▸▾]/g, '').trim());

	it('lists the root, folders marked as folders', async () => {
		await aTree();

		expect(names()).toEqual(['src', 'vendor', 'README.md']);
		expect(rows()[0].getAttribute('aria-expanded')).toBe('false');
		expect(rows()[2].hasAttribute('aria-expanded')).toBe(false);
	});

	it('reads a folder when it is opened, and shows what is in it', async () => {
		await aTree();
		tree.mockClear();

		click(rows()[0]);
		await settle();

		expect(tree).toHaveBeenCalledWith(MAIN, 'src');
		expect(names()).toEqual(['src', 'lib.rs', 'vendor', 'README.md']);
		expect(rows()[0].getAttribute('aria-expanded')).toBe('true');

		click(rows()[0]);
		expect(names()).toEqual(['src', 'vendor', 'README.md']);
	});

	it('opens a file when it is pressed, and marks it as the one open', async () => {
		await aTree();

		click(rows()[2]);
		await settle();

		expect(files.tab?.path).toBe('README.md');
		expect(rows()[2].classList.contains('here')).toBe(true);
	});

	it('does not follow into a submodule', async () => {
		await aTree();

		expect((rows()[1] as HTMLButtonElement).disabled).toBe(true);
		expect(rows()[1].getAttribute('title')).toContain('a submodule');
	});

	it('finds a file by name, and opens what it finds', async () => {
		vi.useFakeTimers();
		findFiles.mockResolvedValue({ paths: ['src/deep/lib.rs', 'src/lib.rs'], more: true });
		await files.enter('/repos/thing', 'main');
		view = render(FileTree, {});
		const field = view.get('.find input') as HTMLInputElement;

		field.value = 'lib';
		fire(field, 'input');
		// Every letter is a walk of the tree, so it waits for a pause.
		expect(findFiles).not.toHaveBeenCalled();
		await vi.advanceTimersByTimeAsync(200);
		vi.useRealTimers();
		await settle();

		expect(findFiles).toHaveBeenCalledWith(MAIN, 'lib');
		expect(view.all('.row.found .name').map((name) => name.textContent)).toEqual(['lib.rs', 'lib.rs']);
		expect(view.all('.row.found .dir').map((dir) => dir.textContent)).toEqual(['src/deep/', 'src/']);
		expect(view.text()).toContain('More match');

		click(view.all('.row.found')[0]);
		await settle();
		expect(files.tab?.path).toBe('src/deep/lib.rs');
	});

	it('says when no file has that name', async () => {
		await aTree();

		await files.findFiles('zzz');
		flushSync();

		expect(view!.text()).toContain('No file by that name');
	});
});

describe('the tree’s right-click (FEAT-152)', () => {
	async function menuOn(name: string): Promise<HTMLElement[]> {
		await files.enter('/repos/thing', 'main');
		await settle();
		view = render(FileTree, {});
		await settle();
		const row = view.all('.rows .row').find((each) => each.textContent?.includes(name))!;
		row.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: 10, clientY: 10 }));
		flushSync();
		return [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')];
	}

	const chosen = (items: HTMLElement[], label: string) => items.find((item) => item.textContent?.includes(label))!;

	it('copies a file’s name, its path from the top, and its text at the revision', async () => {
		const writeText = vi.fn(() => Promise.resolve());
		vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText } });

		click(chosen(await menuOn('README.md'), 'Copy file name'));
		await settle();
		expect(writeText).toHaveBeenLastCalledWith('README.md');

		view?.destroy();
		click(chosen(await menuOn('README.md'), 'Copy relative path'));
		await settle();
		expect(writeText).toHaveBeenLastCalledWith('README.md');

		view?.destroy();
		click(chosen(await menuOn('README.md'), 'Copy file content'));
		await settle();
		expect(read).toHaveBeenLastCalledWith(MAIN, 'README.md');
		expect(writeText).toHaveBeenLastCalledWith(BODY);
	});

	it('offers a folder its name and path, and no content', async () => {
		const items = await menuOn('src');

		expect(items.map((item) => item.textContent)).toEqual([
			expect.stringContaining('Copy folder name'),
			expect.stringContaining('Copy relative path')
		]);
	});
});

describe('a selection longer than the screen (FEAT-152)', () => {
	it('keeps the rows it runs between drawn when they scroll away', async () => {
		const tall = vi.spyOn(HTMLElement.prototype, 'clientHeight', 'get').mockReturnValue(300);
		try {
			const text = Array.from({ length: 3000 }, (_, i) => `line ${i + 1}`).join('\n') + '\n';
			read.mockResolvedValue(
				aFile('long.txt', { grammar: null, colour: false, size: text.length, content: { kind: 'text', text } })
			);
			await opened('long.txt');

			const selection = document.getSelection()!;
			selection.setBaseAndExtent(code(1), 0, code(3), 1);
			document.dispatchEvent(new Event('selectionchange'));
			flushSync();

			keys().scrollTop = 40_000;
			fire(keys(), 'scroll');
			await settle();
			// Both ends, and what runs between them, though line 1 is far above the screen.
			expect(view!.find('.line[data-line="1"]')).not.toBeNull();
			expect(view!.find('.line[data-line="1000"]')).not.toBeNull();

			selection.removeAllRanges();
			document.dispatchEvent(new Event('selectionchange'));
			await settle();
			expect(view!.find('.line[data-line="1"]')).toBeNull();
		} finally {
			tall.mockRestore();
		}
	});
});
