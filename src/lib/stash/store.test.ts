// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { CommitDiff, FileDiff, StashEntry } from '$lib/types';

vi.mock('$lib/api', () => ({
	stashes: vi.fn(),
	stashPush: vi.fn(),
	commitDiff: vi.fn(),
	fileDiff: vi.fn()
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

const actStash = vi.fn<(index: number, name: string, action: string) => Promise<boolean>>();
vi.mock('$lib/graph/actions', () => ({
	stash: (index: number, name: string, action: string) => actStash(index, name, action)
}));

import * as api from '$lib/api';
import { calls as repoCalls, control as repoControl } from '../../testing/repo-store.svelte';
import { activity } from '$lib/activity/store.svelte';
import { stash } from './store.svelte';

const stashes = vi.mocked(api.stashes);
const stashPush = vi.mocked(api.stashPush);
const commitDiff = vi.mocked(api.commitDiff);
const fileDiff = vi.mocked(api.fileDiff);

function entry(index: number, overrides: Partial<StashEntry> = {}): StashEntry {
	const id = `${index}`.padStart(40, 'a');
	return {
		index,
		name: `stash@{${index}}`,
		id,
		short: id.slice(0, 7),
		message: `On main: entry ${index}`,
		time: 1_700_000_000 - index * 60,
		authorName: 'Ada Lovelace',
		parent: 'b'.repeat(40),
		parentShort: 'bbbbbbb',
		parentSummary: 'Merge feature/split-view',
		...overrides
	};
}

function change(path: string) {
	return {
		path,
		status: 'modified' as const,
		binary: false,
		tooLarge: false,
		added: 1,
		removed: 0
	};
}

function diff(id: string, paths: string[] = ['notes.md']): CommitDiff {
	return {
		id,
		short: id.slice(0, 7),
		summary: 'a stash',
		files: paths.map(change),
		added: paths.length,
		removed: 0
	};
}

function fileOf(path: string): FileDiff {
	return { ...change(path), hunks: [] };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((res) => (resolve = res));
	return { promise, resolve };
}

beforeEach(() => {
	vi.clearAllMocks();
	stash.clear();
	repoControl.reset();
	stashes.mockResolvedValue([entry(0), entry(1)]);
	commitDiff.mockImplementation((id: string) => Promise.resolve(diff(id)));
	fileDiff.mockImplementation((_id: string, path: string) => Promise.resolve(fileOf(path)));
	stashPush.mockResolvedValue(undefined);
});

describe('load', () => {
	it('lists the entries and opens the newest', async () => {
		await stash.load();
		await settle();

		expect(stash.entries.map((e) => e.name)).toEqual(['stash@{0}', 'stash@{1}']);
		expect(stash.selected?.index).toBe(0);
		expect(stash.contents?.files).toHaveLength(1);
	});

	it('reads an entry through the commit diff, because a stash is a commit', async () => {
		await stash.load();
		await settle();

		expect(commitDiff).toHaveBeenCalledWith(entry(0).id);
	});

	it('selects nothing when there is no stash', async () => {
		stashes.mockResolvedValueOnce([]);
		await stash.load();
		await settle();

		expect(stash.entries).toEqual([]);
		expect(stash.selected).toBeNull();
		expect(commitDiff).not.toHaveBeenCalled();
	});

	it('records a failure and shows nothing', async () => {
		stashes.mockRejectedValueOnce('could not read refs');
		await stash.load();

		expect(stash.error).toBe('could not read refs');
		expect(stash.entries).toEqual([]);
		expect(stash.selected).toBeNull();
	});

	it('keeps the open entry across a reload when it survived', async () => {
		await stash.load();
		await settle();
		stash.select(entry(1).id);
		await settle();

		await stash.load();
		await settle();

		expect(stash.selected?.index).toBe(1);
	});

	it('falls back to the newest when the open entry is gone', async () => {
		await stash.load();
		await settle();
		stash.select(entry(1).id);
		await settle();

		stashes.mockResolvedValueOnce([entry(0)]);
		await stash.load();
		await settle();

		expect(stash.selected?.index).toBe(0);
	});

	it('drops a slow read that a newer one superseded', async () => {
		const slow = deferred<StashEntry[]>();
		stashes.mockReturnValueOnce(slow.promise);

		const first = stash.load();
		await stash.load();
		slow.resolve([entry(9, { message: 'stale' })]);
		await first;
		await settle();

		expect(stash.entries.map((e) => e.message)).not.toContain('stale');
	});
});

describe('select', () => {
	it('does not re-read an entry that is already open', async () => {
		await stash.load();
		await settle();
		commitDiff.mockClear();

		stash.select(entry(0).id);
		expect(commitDiff).not.toHaveBeenCalled();
	});

	it('records a failure to read an entry without losing the list', async () => {
		await stash.load();
		await settle();

		commitDiff.mockRejectedValueOnce('no commit');
		stash.select(entry(1).id);
		await settle();

		expect(stash.contentsError).toBe('no commit');
		expect(stash.contentsLoading).toBe(false);
		expect(stash.entries).toHaveLength(2);
	});

	it('drops a slow read that a newer selection superseded', async () => {
		await stash.load();
		await settle();

		const slow = deferred<CommitDiff>();
		commitDiff.mockReturnValueOnce(slow.promise);

		stash.select(entry(1).id);
		stash.select(entry(0).id);
		await settle();

		slow.resolve(diff(entry(1).id));
		await settle();

		expect(stash.contents?.id).toBe(entry(0).id);
	});
});

describe('push', () => {
	it('stashes, clears the message and re-reads everything', async () => {
		await stash.load();
		await settle();
		stash.setMessage('wip');
		stashes.mockClear();

		expect(await stash.push()).toBe(true);

		expect(stashPush).toHaveBeenCalledWith('wip', false);
		expect(stash.message).toBe('');
		expect(stashes).toHaveBeenCalled();
		expect(repoCalls.refreshed).toBeGreaterThan(0);
	});

	it('can take untracked files too', async () => {
		await stash.load();
		await settle();
		stash.setIncludeUntracked(true);

		await stash.push();

		expect(stashPush).toHaveBeenCalledWith('', true);
	});

	it('keeps the message when the stash is refused', async () => {
		await stash.load();
		await settle();
		stash.setMessage('wip');
		stashPush.mockRejectedValueOnce('there is nothing to stash');

		expect(await stash.push()).toBe(false);

		expect(stash.message).toBe('wip');
		expect(stash.writeError).toBe('there is nothing to stash');
	});

	it('refuses a second push while one is in flight', async () => {
		await stash.load();
		await settle();
		const slow = deferred<void>();
		stashPush.mockReturnValueOnce(slow.promise);

		const first = stash.push();
		expect(await stash.push()).toBe(false);
		expect(stashPush).toHaveBeenCalledTimes(1);

		slow.resolve();
		await first;
	});
});

describe('push, said while it runs (FEAT-121)', () => {
	it('names the stash for as long as git takes', async () => {
		activity.reset();
		let finish!: () => void;
		stashPush.mockReturnValueOnce(new Promise<void>((resolve) => (finish = resolve)));

		const running = stash.push();
		expect(activity.label).toBe('Stashing your changes…');
		expect(activity.is('stash')).toBe(true);

		finish();
		await running;
		expect(activity.running).toBe(false);
	});

	it('does not stash over a write still running, and says which', async () => {
		activity.reset();
		const step = activity.begin('checkout', 'Switching to main');

		expect(await stash.push()).toBe(false);

		expect(stashPush).not.toHaveBeenCalled();
		expect(stash.writeError).toBe('Switching to main is still running');
		step?.end();
	});
});

describe('clear', () => {
	it('forgets the entries, the selection and the message', async () => {
		await stash.load();
		await settle();
		stash.setMessage('half-typed');
		stash.setIncludeUntracked(true);

		stash.clear();

		expect(stash.entries).toEqual([]);
		expect(stash.loaded).toBe(false);
		expect(stash.selected).toBeNull();
		expect(stash.contents).toBeNull();
		expect(stash.message).toBe('');
		expect(stash.includeUntracked).toBe(false);
	});
});

/**
 * FEAT-014 — restoring an entry.
 *
 * The confirmation and the write are `graph/actions.ts`'s, and are tested
 * there. What is this store's own is the part `actions` cannot know about: the
 * list on this screen is now stale, and `perform`'s refresh reaches the graph
 * and the rail but not here.
 */
describe('restore', () => {
	async function withOneEntry() {
		stashes.mockResolvedValue([entry(0), entry(1)]);
		commitDiff.mockResolvedValue({ files: [] } as unknown as CommitDiff);
		await stash.load();
		stashes.mockClear();
		repoCalls.refreshed = 0;
	}

	it('hands the selected entry’s index and name to the action', async () => {
		await withOneEntry();
		actStash.mockResolvedValue(true);

		await stash.restore('pop');

		expect(actStash).toHaveBeenCalledWith(0, 'stash@{0}', 'pop');
	});

	it('re-reads the list and the rail once the entry is gone', async () => {
		await withOneEntry();
		actStash.mockResolvedValue(true);
		stashes.mockResolvedValue([entry(1)]);

		await stash.restore('pop');

		expect(stashes).toHaveBeenCalledTimes(1);
		expect(repoCalls.refreshed).toBe(1);
	});

	/** A cancelled confirmation changed nothing, so there is nothing to re-read. */
	it('does not re-read when the action reports it did nothing', async () => {
		await withOneEntry();
		actStash.mockResolvedValue(false);

		await stash.restore('drop');

		expect(stashes).not.toHaveBeenCalled();
		expect(repoCalls.refreshed).toBe(0);
	});

	/**
	 * Pop and drop remove the entry, so the selection is released before the
	 * re-read rather than left pointing at something that no longer exists.
	 * Apply keeps it, so the open entry stays open.
	 */
	it('releases the selection for pop and drop, keeps it for apply', async () => {
		for (const action of ['pop', 'drop'] as const) {
			await withOneEntry();
			actStash.mockResolvedValue(true);
			stashes.mockResolvedValue([]);

			await stash.restore(action);
			expect(stash.selected, action).toBeNull();
		}

		await withOneEntry();
		actStash.mockResolvedValue(true);
		stashes.mockResolvedValue([entry(0), entry(1)]);

		await stash.restore('apply');
		expect(stash.selected?.name).toBe('stash@{0}');
	});

	it('does nothing with no entry selected', async () => {
		stash.clear();

		await stash.restore('pop');

		expect(actStash).not.toHaveBeenCalled();
	});

	/** Two clicks on Drop must not drop two entries. */
	it('refuses to run a second restore while one is in flight', async () => {
		await withOneEntry();
		let release: (value: boolean) => void = () => {};
		actStash.mockReturnValueOnce(new Promise<boolean>((resolve) => (release = resolve)));

		const first = stash.restore('drop');
		const second = stash.restore('drop');

		expect(actStash).toHaveBeenCalledTimes(1);

		release(false);
		await first;
		await second;
	});

	it('clears busy even when the action throws', async () => {
		await withOneEntry();
		actStash.mockRejectedValueOnce(new Error('boom'));

		await expect(stash.restore('pop')).rejects.toThrow('boom');
		expect(stash.busy).toBe(false);
	});
});

/**
 * FEAT-034. A stash is a commit, so one of its files is `fileDiff` on the
 * entry's id — the same read the Diff screen makes, against the same command.
 */
describe('browsing an entry file by file', () => {
	const threeFiles = ['a.txt', 'b.txt', 'c.txt'];

	async function withThreeFiles() {
		commitDiff.mockImplementation((id: string) => Promise.resolve(diff(id, threeFiles)));
		await stash.load();
		await settle();
	}

	it('opens an entry on its first file rather than on an empty pane', async () => {
		await withThreeFiles();

		expect(stash.path).toBe('a.txt');
		expect(stash.file?.path).toBe('a.txt');
		expect(fileDiff).toHaveBeenCalledWith(entry(0).id, 'a.txt');
	});

	it('reads a file against the entry it belongs to', async () => {
		await withThreeFiles();
		stash.selectFile('b.txt');
		await settle();

		expect(fileDiff).toHaveBeenLastCalledWith(entry(0).id, 'b.txt');
		expect(stash.file?.path).toBe('b.txt');
	});

	it('says where in the entry the open file is', async () => {
		await withThreeFiles();
		expect(stash.fileIndex).toBe(0);
		expect(stash.fileCount).toBe(3);

		stash.selectFile('c.txt');
		await settle();

		expect(stash.fileIndex).toBe(2);
	});

	it('walks the files and stops at either end', async () => {
		await withThreeFiles();

		stash.stepFile(1);
		await settle();
		expect(stash.path).toBe('b.txt');

		stash.stepFile(-1);
		await settle();
		expect(stash.path).toBe('a.txt');

		// Past the start is the start, not an error and not a wrap.
		stash.stepFile(-1);
		await settle();
		expect(stash.path).toBe('a.txt');

		stash.stepFile(99);
		await settle();
		expect(stash.path).toBe('c.txt');

		stash.stepFile(99);
		await settle();
		expect(stash.path).toBe('c.txt');
	});

	it('re-reads nothing when a file is opened twice', async () => {
		await withThreeFiles();
		stash.selectFile('b.txt');
		await settle();
		fileDiff.mockClear();

		stash.selectFile('a.txt');
		await settle();
		stash.selectFile('b.txt');
		await settle();

		expect(fileDiff).not.toHaveBeenCalled();
		expect(stash.file?.path).toBe('b.txt');
	});

	it('throws the cache away when a different entry is opened', async () => {
		await withThreeFiles();
		fileDiff.mockClear();

		stash.select(entry(1).id);
		await settle();

		// The path is the same string; the file it names belongs to another
		// entry, so it is read again rather than served from the last one.
		expect(fileDiff).toHaveBeenCalledWith(entry(1).id, 'a.txt');
	});

	it('keeps the open file when the same entry is read again', async () => {
		// A re-read happens after an apply and after `repo-changed`. Jumping
		// back to the first file every time would lose the reader's place.
		await withThreeFiles();
		stash.selectFile('c.txt');
		await settle();

		await stash.load();
		await settle();

		expect(stash.path).toBe('c.txt');
	});

	it('opens the first file when the one that was open is gone', async () => {
		await withThreeFiles();
		stash.selectFile('c.txt');
		await settle();

		commitDiff.mockImplementation((id: string) => Promise.resolve(diff(id, ['a.txt', 'b.txt'])));
		await stash.load();
		await settle();

		expect(stash.path).toBe('a.txt');
	});

	it('selects nothing at all for an entry that changed nothing', async () => {
		commitDiff.mockImplementation((id: string) => Promise.resolve(diff(id, [])));
		await stash.load();
		await settle();

		expect(stash.path).toBeNull();
		expect(stash.fileIndex).toBe(-1);
		expect(stash.fileCount).toBe(0);
		expect(fileDiff).not.toHaveBeenCalled();

		// And stepping through nothing is a no-op rather than a crash.
		stash.stepFile(1);
		expect(stash.path).toBeNull();
	});

	it('surfaces a failed file read without losing the file list', async () => {
		await withThreeFiles();
		fileDiff.mockRejectedValueOnce('no file b.txt in that commit');

		stash.selectFile('b.txt');
		await settle();

		expect(stash.fileError).toBe('no file b.txt in that commit');
		expect(stash.file).toBeNull();
		expect(stash.contents?.files).toHaveLength(3);
	});

	it('ignores a slow read that lost the race to a newer one', async () => {
		await withThreeFiles();
		const slow = deferred<FileDiff>();
		fileDiff.mockReturnValueOnce(slow.promise);

		stash.selectFile('b.txt');
		stash.selectFile('c.txt');
		await settle();

		slow.resolve(fileOf('b.txt'));
		await settle();

		expect(stash.path).toBe('c.txt');
		expect(stash.file?.path).toBe('c.txt');
	});

	it('forgets the open file when the screen is cleared', async () => {
		await withThreeFiles();
		stash.clear();

		expect(stash.path).toBeNull();
		expect(stash.file).toBeNull();
		expect(stash.fileCount).toBe(0);
	});

	it('does nothing when a file is selected with no entry open', () => {
		stash.selectFile('a.txt');
		expect(fileDiff).not.toHaveBeenCalled();
		expect(stash.path).toBeNull();
	});
});
