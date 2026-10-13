// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BlameCommit, CompactBlame, FilesEntry, FilesRead } from '$lib/types';

vi.mock('$lib/api', () => ({
	filesResolve: vi.fn(),
	filesTree: vi.fn(),
	filesRead: vi.fn(),
	filesFind: vi.fn(),
	filesSpans: vi.fn(),
	filesBlame: vi.fn(),
	filesBefore: vi.fn()
}));

import * as api from '$lib/api';
import { COLOUR_PAGE, files, pullOf, revisionLabel } from './store.svelte';

const resolve = vi.mocked(api.filesResolve);
const tree = vi.mocked(api.filesTree);
const read = vi.mocked(api.filesRead);
const findFiles = vi.mocked(api.filesFind);
const spans = vi.mocked(api.filesSpans);
const blame = vi.mocked(api.filesBlame);
const before = vi.mocked(api.filesBefore);

const REPO = '/repos/thing';
const IDS: Record<string, string> = { main: 'a'.repeat(40), side: 'b'.repeat(40), HEAD: 'a'.repeat(40) };

function entry(path: string, kind: FilesEntry['kind'] = 'file'): FilesEntry {
	return { path, name: path.slice(path.lastIndexOf('/') + 1), kind };
}

function text(path: string, body: string, overrides: Partial<FilesRead> = {}): FilesRead {
	return {
		path,
		revision: IDS.main,
		blob: `blob-${path}`,
		size: body.length,
		grammar: 'rust',
		colour: true,
		content: { kind: 'text', text: body },
		...overrides
	};
}

function aCommit(overrides: Partial<BlameCommit> = {}): BlameCommit {
	return {
		id: 'c'.repeat(40),
		short: 'ccccccc',
		summary: 'Add',
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

function aBlame(lines: number[], commits: BlameCommit[]): CompactBlame {
	return {
		path: 'src/lib.rs',
		revision: IDS.main,
		commits,
		lines,
		origin: lines.map((_, index) => index + 1),
		refused: null
	};
}

const BODY = 'one\ntwo\nthree\nfour\n';

/** Let promises settle and the after-paint work run. */
async function settle(): Promise<void> {
	for (let i = 0; i < 6; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
}

beforeEach(() => {
	vi.clearAllMocks();
	localStorage.clear();
	files.clearState();
	// The frame that paints is a timer here, so "after paint" is a tick away.
	vi.stubGlobal('requestAnimationFrame', (work: FrameRequestCallback) => setTimeout(() => work(0), 0));

	resolve.mockImplementation(async (rev) => {
		const id = IDS[rev];
		if (!id) throw new Error(`no commit ${rev}`);
		return { id, short: id.slice(0, 7) };
	});
	tree.mockImplementation(async (_rev, dir) =>
		dir === '' ? [entry('src', 'folder'), entry('README.md')] : [entry(`${dir}/lib.rs`)]
	);
	read.mockImplementation(async (_rev, path) => text(path, BODY));
	spans.mockResolvedValue([]);
	blame.mockResolvedValue(aBlame([0, 0, 0, 0], [aCommit()]));
	before.mockResolvedValue(null);
	findFiles.mockResolvedValue({ paths: [], more: false });
});

describe('entering a repository', () => {
	it('reads the tree at the branch that is checked out', async () => {
		await files.enter(REPO, 'main');

		expect(files.rev).toBe('main');
		expect(files.resolved?.id).toBe(IDS.main);
		expect(tree).toHaveBeenCalledWith(IDS.main, '');
		expect(files.folder('')).toEqual([entry('src', 'folder'), entry('README.md')]);
		expect(files.tabs).toEqual([]);
		expect(files.file).toBeNull();
	});

	it('reads HEAD when nothing is checked out by name', async () => {
		await files.enter(REPO, null);

		expect(files.rev).toBe('HEAD');
	});

	it('only looks for a moved branch when it is the same repository again', async () => {
		await files.enter(REPO, 'main');
		await files.openFile('src/lib.rs');
		tree.mockClear();
		read.mockClear();

		await files.enter(REPO, 'main');

		expect(tree).not.toHaveBeenCalled();
		expect(read).not.toHaveBeenCalled();
		expect(files.tabs).toHaveLength(1);
	});

	it('reads the tree and the open file again when the branch has moved', async () => {
		await files.enter(REPO, 'main');
		await files.openFile('src/lib.rs');
		tree.mockClear();
		read.mockClear();
		IDS.main = 'd'.repeat(40);

		await files.refresh();

		expect(files.resolved?.id).toBe('d'.repeat(40));
		expect(tree).toHaveBeenCalledWith('d'.repeat(40), '');
		expect(read).toHaveBeenCalledTimes(1);
		IDS.main = 'a'.repeat(40);
	});
});

describe('the tree', () => {
	it('reads a folder the first time it is opened, and not again', async () => {
		await files.enter(REPO, 'main');
		tree.mockClear();

		files.toggleFolder('src');
		await settle();

		expect(tree).toHaveBeenCalledWith(IDS.main, 'src');
		expect(files.open).toEqual(['src']);
		expect(files.folder('src')).toEqual([entry('src/lib.rs')]);

		files.toggleFolder('src');
		files.toggleFolder('src');
		await settle();
		expect(tree).toHaveBeenCalledTimes(1);
	});

	it('opens every folder down to the one asked for', async () => {
		await files.enter(REPO, 'main');

		files.reveal('crates/core/src');
		await settle();

		expect(files.open).toEqual(['crates', 'crates/core', 'crates/core/src']);
	});

	it('shows where a file is, however it was opened', async () => {
		await files.enter(REPO, 'main');

		await files.openFile('crates/core/src/lib.rs');

		expect(files.open).toEqual(['crates', 'crates/core', 'crates/core/src']);
	});

	it('closes a folder the revision does not have', async () => {
		await files.enter(REPO, 'main');
		tree.mockRejectedValueOnce('no file gone in that commit');

		files.toggleFolder('gone');
		await settle();

		expect(files.open).not.toContain('gone');
		expect(files.folder('gone')).toEqual({ error: 'no file gone in that commit' });
	});

	it('looks through every path for a word, and comes back to the tree without one', async () => {
		await files.enter(REPO, 'main');
		findFiles.mockResolvedValue({ paths: ['src/lib.rs'], more: false });

		await files.findFiles('lib');
		expect(findFiles).toHaveBeenCalledWith(IDS.main, 'lib');
		expect(files.found?.paths).toEqual(['src/lib.rs']);

		await files.findFiles('  ');
		expect(files.found).toBeNull();
	});
});

describe('opening a file', () => {
	beforeEach(async () => {
		await files.enter(REPO, 'main');
	});

	it('opens it in a tab and reads its lines', async () => {
		await files.openFile('src/lib.rs');

		expect(files.tabs).toEqual([{ path: 'src/lib.rs', rev: 'main', line: 0 }]);
		expect(files.active).toBe(0);
		expect(read).toHaveBeenCalledWith('main', 'src/lib.rs');
		expect(files.lines).toEqual(['one', 'two', 'three', 'four']);
		expect(files.cursor).toEqual({ line: 0, col: 0 });
	});

	it('goes back to the tab a file already has', async () => {
		await files.openFile('src/lib.rs');
		await files.openFile('README.md');
		await files.openFile('src/lib.rs');

		expect(files.tabs.map((tab) => tab.path)).toEqual(['src/lib.rs', 'README.md']);
		expect(files.active).toBe(0);
		expect(files.file?.path).toBe('src/lib.rs');
	});

	it('opens at a line, and at the revision another screen was showing', async () => {
		await files.openFile('src/lib.rs', { rev: 'side', line: 3 });

		expect(files.rev).toBe('side');
		expect(files.resolved?.id).toBe(IDS.side);
		expect(read).toHaveBeenCalledWith('side', 'src/lib.rs');
		expect(files.cursor.line).toBe(2);
	});

	it('goes to a line of the file already on screen without reading it again', async () => {
		await files.openFile('src/lib.rs');
		read.mockClear();

		await files.openFile('src/lib.rs', { line: 4 });

		expect(read).not.toHaveBeenCalled();
		expect(files.cursor.line).toBe(3);
	});

	it('remembers where each tab was read to', async () => {
		await files.openFile('src/lib.rs');
		files.moveTo(2);
		await files.openFile('README.md');
		await files.activate(0);

		expect(files.cursor.line).toBe(2);
	});

	it('closes a tab and shows the one beside it', async () => {
		await files.openFile('src/lib.rs');
		await files.openFile('README.md');

		await files.close(1);
		expect(files.tabs.map((tab) => tab.path)).toEqual(['src/lib.rs']);
		expect(files.file?.path).toBe('src/lib.rs');

		await files.close(0);
		expect(files.tabs).toEqual([]);
		expect(files.file).toBeNull();
	});

	/** Five files clicked in a second: the second's answer must not land on the fifth. */
	it('drops an answer for a file that is no longer the one asked for', async () => {
		let slow: (value: FilesRead) => void = () => {};
		read.mockImplementationOnce(() => new Promise((done) => (slow = done)));

		const first = files.openFile('slow.rs');
		await settle();
		await files.openFile('README.md');
		slow(text('slow.rs', 'late\n'));
		await first;

		expect(files.file?.path).toBe('README.md');
		expect(files.lines).toEqual(['one', 'two', 'three', 'four']);
	});

	it('says why a file could not be read', async () => {
		read.mockRejectedValueOnce('could not diff: object not found');

		await files.openFile('src/lib.rs');

		expect(files.fileError).toContain('object not found');
		expect(files.reading).toBe(false);
	});

	it('has no lines for a file that is not text', async () => {
		read.mockResolvedValueOnce(text('data.bin', '', { content: { kind: 'binary' }, grammar: null }));

		await files.openFile('data.bin');
		await settle();

		expect(files.lines).toEqual([]);
		expect(blame).not.toHaveBeenCalled();
		expect(files.reference).toBeNull();
	});
});

describe('the revision', () => {
	beforeEach(async () => {
		await files.enter(REPO, 'main');
		await files.openFile('src/lib.rs');
		await settle();
		vi.clearAllMocks();
	});

	it('reads the tree and the open file at another revision', async () => {
		await files.setRevision('side');

		expect(files.rev).toBe('side');
		expect(tree).toHaveBeenCalledWith(IDS.side, '');
		expect(read).toHaveBeenCalledWith('side', 'src/lib.rs');
		expect(files.tabs[0].rev).toBe('side');
	});

	it('stays where it was and says so when the word is not a revision', async () => {
		await files.setRevision('nowhere');

		expect(files.rev).toBe('main');
		expect(files.resolved?.id).toBe(IDS.main);
		expect(files.revisionError).toContain('nowhere');
		expect(read).not.toHaveBeenCalled();
	});

	it('follows a tab that was opened at another revision', async () => {
		await files.openFile('README.md', { rev: 'side' });
		expect(files.rev).toBe('side');

		await files.activate(0);

		expect(files.rev).toBe('main');
		expect(files.resolved?.id).toBe(IDS.main);
	});
});

describe('the order things arrive in', () => {
	beforeEach(async () => {
		await files.enter(REPO, 'main');
	});

	/** The item: opening paints text before colour and blame. */
	it('has the text before the blame is even asked for', async () => {
		await files.openFile('src/lib.rs');

		expect(files.lines).toHaveLength(4);
		expect(blame).not.toHaveBeenCalled();
		expect(spans).not.toHaveBeenCalled();

		await settle();
		expect(blame).toHaveBeenCalledWith(IDS.main, 'src/lib.rs');
		expect(files.blame?.lines).toEqual([0, 0, 0, 0]);
	});

	it('asks for colour only for the rows about to be drawn, a page at a time', async () => {
		const long = Array.from({ length: COLOUR_PAGE * 3 }, (_, n) => `line ${n}`).join('\n');
		read.mockResolvedValueOnce(text('long.rs', long));
		await files.openFile('long.rs');

		files.wantColour(0, 60);
		expect(spans).toHaveBeenCalledTimes(1);
		expect(spans).toHaveBeenCalledWith('blob-long.rs', 'rust', 0, COLOUR_PAGE);

		// The same rows again ask for nothing.
		files.wantColour(10, 70);
		expect(spans).toHaveBeenCalledTimes(1);

		// Across a page's edge: only the page not asked for yet.
		files.wantColour(COLOUR_PAGE - 10, COLOUR_PAGE + 10);
		expect(spans).toHaveBeenCalledTimes(2);
		expect(spans).toHaveBeenLastCalledWith('blob-long.rs', 'rust', COLOUR_PAGE, COLOUR_PAGE * 2);
	});

	it('colours a line once its runs arrive, and draws it plain until then', async () => {
		spans.mockResolvedValueOnce([[1, 3], []]);
		await files.openFile('src/lib.rs');

		expect(files.tokens(0)).toEqual([{ type: 'plain', text: 'one' }]);
		files.wantColour(0, 4);
		await settle();

		expect(files.tokens(0)).toEqual([{ type: 'keyword', text: 'one' }]);
		expect(files.tokens(1)).toEqual([{ type: 'plain', text: 'two' }]);
	});

	it('asks for colour again when the first asking failed', async () => {
		spans.mockRejectedValueOnce('no grammar');
		await files.openFile('src/lib.rs');

		files.wantColour(0, 4);
		await settle();
		files.wantColour(0, 4);

		expect(spans).toHaveBeenCalledTimes(2);
	});

	it('asks tree-sitter for nothing when the file is too large to colour', async () => {
		read.mockResolvedValueOnce(text('big.rs', BODY, { colour: false, grammar: null }));
		await files.openFile('big.rs');

		files.wantColour(0, 4);
		await settle();

		expect(spans).not.toHaveBeenCalled();
		expect(files.tokens(0)).toEqual([{ type: 'plain', text: 'one' }]);
	});

	/** No grammar compiled in: the highlighter Diff uses reads it instead. */
	it('colours a language tree-sitter has no grammar for with the other highlighter', async () => {
		read.mockResolvedValueOnce(text('Main.kt', 'fun main() {}\n', { grammar: null }));
		await files.openFile('Main.kt');

		expect(files.treeSitter).toBe(false);
		expect(files.language).toBe('kotlin');
		await settle();

		expect(spans).not.toHaveBeenCalled();
		expect(files.tokens(0)[0]).toEqual({ type: 'keyword', text: 'fun' });
	});

	it('does not blame while the gutter is off, and does when it is turned on', async () => {
		files.toggleBlame();
		expect(files.blameOn).toBe(false);
		await files.openFile('src/lib.rs');
		await settle();
		expect(blame).not.toHaveBeenCalled();

		files.toggleBlame();
		await settle();
		expect(blame).toHaveBeenCalledTimes(1);
	});
});

describe('blame and the peek', () => {
	const agent = aCommit({
		id: 'e'.repeat(40),
		short: 'eeeeeee',
		summary: 'Cache on disk (#214)',
		agent: 'Codex',
		previous: 'c'.repeat(40),
		previousPath: 'src/old.rs'
	});

	beforeEach(async () => {
		blame.mockResolvedValue(aBlame([0, 1, 1, 0], [aCommit(), agent]));
		await files.enter(REPO, 'main');
		await files.openFile('src/lib.rs');
		await settle();
	});

	it('knows the commit that wrote each line', () => {
		expect(files.commitOf(0)?.agent).toBeNull();
		expect(files.commitOf(1)?.agent).toBe('Codex');
		expect(files.commitOf(99)).toBeNull();
	});

	it('lights every line a commit wrote, and puts them out on the same block again', () => {
		files.light(1);
		expect(files.lit).toBe(1);
		files.light(2);
		expect(files.lit).toBeNull();
		files.light(0);
		expect(files.lit).toBe(0);
	});

	it('steps from one blame block to the next', () => {
		files.moveBlock(1);
		expect(files.cursor.line).toBe(1);
		files.moveBlock(1);
		expect(files.cursor.line).toBe(3);
		files.moveBlock(-1);
		expect(files.cursor.line).toBe(1);
	});

	it('reads what the line was before, from the commit it was changed from', async () => {
		before.mockResolvedValueOnce('    old line');

		files.openPeek(1);
		expect(files.peek).toBe(1);
		expect(files.was).toBeUndefined();
		await settle();

		expect(before).toHaveBeenCalledWith('e'.repeat(40), 'src/lib.rs', 2, 'c'.repeat(40), 'src/old.rs');
		expect(files.was).toBe('    old line');
	});

	it('closes the peek when the cursor leaves its line', () => {
		files.openPeek(1);
		files.moveBy(1);

		expect(files.peek).toBeNull();
	});

	it('asks for the blame when the peek is opened with the gutter off', async () => {
		files.toggleBlame();
		await files.openFile('README.md');
		await settle();
		blame.mockClear();

		files.openPeek(0);
		await settle();

		expect(blame).toHaveBeenCalledTimes(1);
		expect(files.commitOf(0)).not.toBeNull();
	});

	it('blames again at the commit before, under the name the file had then', async () => {
		IDS['c'.repeat(40)] = 'c'.repeat(40);

		await files.blameBefore(1);

		expect(files.rev).toBe('c'.repeat(40));
		expect(files.tab?.path).toBe('src/old.rs');
		expect(read).toHaveBeenLastCalledWith('c'.repeat(40), 'src/old.rs');
		delete IDS['c'.repeat(40)];
	});

	it('has no blame before for the commit that added the file', async () => {
		read.mockClear();

		await files.blameBefore(0);

		expect(read).not.toHaveBeenCalled();
	});

	it('says why there is no blame', async () => {
		blame.mockRejectedValueOnce('git blame failed: fatal');
		await files.openFile('README.md');
		await settle();

		expect(files.blameError).toContain('git blame failed');
		expect(files.blaming).toBe(false);
	});
});

describe('moving, selecting and searching', () => {
	beforeEach(async () => {
		read.mockImplementation(async (_rev, path) => text(path, 'alpha\nbeta\ngamma beta\ndelta\n'));
		await files.enter(REPO, 'main');
		await files.openFile('src/lib.rs');
	});

	it('keeps the cursor inside the file', () => {
		files.moveBy(-5);
		expect(files.cursor.line).toBe(0);
		files.moveBy(99);
		expect(files.cursor.line).toBe(3);
	});

	it('selects a range with shift, and drops it on an ordinary move', () => {
		files.moveTo(1);
		files.moveTo(3, true);
		expect(files.selection).toEqual([1, 3]);

		files.moveTo(0, true);
		expect(files.selection).toEqual([0, 1]);

		files.moveTo(2);
		expect(files.selection).toEqual([2, 2]);
	});

	it('copies where you are as path:line@sha', () => {
		files.moveTo(1);
		expect(files.reference).toBe(`src/lib.rs:2@${'a'.repeat(7)}`);

		files.moveTo(3, true);
		expect(files.reference).toBe(`src/lib.rs:2-4@${'a'.repeat(7)}`);
	});

	it('searches forward from the cursor and goes on with n', () => {
		files.startSearch(false);
		files.setQuery('beta');
		expect(files.searching).toBe(true);

		expect(files.searchNext()).toEqual({ line: 1, start: 0, end: 4 });
		files.endSearch();
		expect(files.cursor).toEqual({ line: 1, col: 0 });
		expect(files.searchNext()).toEqual({ line: 2, start: 6, end: 10 });
		expect(files.cursor).toEqual({ line: 2, col: 6 });
		// N turns round.
		expect(files.searchNext(true)).toEqual({ line: 1, start: 0, end: 4 });
	});

	it('searches backward with ?, where N goes forward', () => {
		files.moveTo(3);
		files.startSearch(true);
		files.setQuery('beta');

		expect(files.searchNext()?.line).toBe(2);
		expect(files.searchNext(true)?.line).toBe(1);
	});

	it('says when a word is nowhere', () => {
		files.startSearch(false);
		files.setQuery('omega');

		expect(files.searchNext()).toBeNull();
		expect(files.nowhere).toBe(true);
		expect(files.cursor.line).toBe(0);
	});

	it('puts out what is nearest first: the peek, the match, the range, the lit commit', async () => {
		await settle();
		files.light(0);
		files.setQuery('beta');
		files.searchNext();
		files.moveTo(3, true);
		files.openPeek(1);

		files.dismiss();
		expect(files.peek).toBeNull();
		expect(files.lit).toBe(0);
		files.dismiss();
		files.dismiss();
		files.dismiss();
		expect(files.lit).toBeNull();
	});
});

describe('what is remembered', () => {
	it('keeps the revision, the tabs, the open folders and the switches per repository', async () => {
		await files.enter(REPO, 'main');
		await files.openFile('src/lib.rs');
		await files.openFile('README.md');
		files.toggleWrap();
		files.toggleBlame();
		await files.setRevision('side');

		files.clearState();
		await files.enter(REPO, 'main');

		expect(files.rev).toBe('side');
		expect(files.tabs.map((tab) => tab.path)).toEqual(['src/lib.rs', 'README.md']);
		expect(files.active).toBe(1);
		expect(files.open).toEqual(['src']);
		expect(files.wrap).toBe(true);
		expect(files.blameOn).toBe(false);
		expect(files.file?.path).toBe('README.md');
	});

	it('keeps one repository’s tabs out of another’s', async () => {
		await files.enter(REPO, 'main');
		await files.openFile('src/lib.rs');

		await files.enter('/repos/other', 'main');

		expect(files.tabs).toEqual([]);
		expect(files.file).toBeNull();
	});

	it('goes back to where HEAD is when the remembered branch has gone', async () => {
		localStorage.setItem(
			'spagitty.files',
			JSON.stringify({ [REPO]: { rev: 'deleted', tabs: [], active: -1, open: [], blame: true, wrap: false } })
		);

		await files.enter(REPO, 'main');

		expect(files.rev).toBe('main');
		expect(files.resolved?.id).toBe(IDS.main);
	});

	it('opens with nothing remembered when what was stored cannot be read', async () => {
		localStorage.setItem('spagitty.files', '{not json');

		await files.enter(REPO, 'main');

		expect(files.rev).toBe('main');
		expect(files.tabs).toEqual([]);
	});
});

describe('labels', () => {
	it('cuts a commit’s full id to seven and leaves a name alone', () => {
		expect(revisionLabel('a'.repeat(40))).toBe('aaaaaaa');
		expect(revisionLabel('main')).toBe('main');
		expect(revisionLabel('release/2.2')).toBe('release/2.2');
		expect(revisionLabel('abc1234')).toBe('abc1234');
	});

	it('reads the pull request a summary names, as a host writes it', () => {
		expect(pullOf('Cache avatars on disk (#214)')).toBe(214);
		expect(pullOf('Merge pull request #69 from spa-git-ty/release/2.2.2')).toBe(69);
		expect(pullOf('Fix #12 in the cache')).toBeNull();
		expect(pullOf('Add avatar cache')).toBeNull();
	});
});
