// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { LifeAsk, LifeEntry, LifePage } from '$lib/types';
import { BRANCH, BRANCH_FACTS, entry, FILE, FOLDER, HUNK, sha, TODAY } from '../../testing/life-fixtures';

vi.mock('$lib/api', () => ({
	investigateLife: vi.fn(),
	investigateToday: vi.fn(),
	investigateHunk: vi.fn(),
	investigateCompare: vi.fn(),
	investigateBranch: vi.fn()
}));

import * as api from '$lib/api';
import { investigate, LIFE_MAX, PAGE } from './store.svelte';

const life = vi.mocked(api.investigateLife);
const today = vi.mocked(api.investigateToday);
const hunk = vi.mocked(api.investigateHunk);
const compare = vi.mocked(api.investigateCompare);
const branch = vi.mocked(api.investigateBranch);

const AVATARS = { kind: 'file', path: 'core/avatars.rs' } as const;

/** Serve `all` a page at a time, as the backend does. */
function serve(all: LifeEntry[]) {
	life.mockImplementation(async (ask: LifeAsk): Promise<LifePage> => {
		const entries = all.slice(ask.skip, ask.skip + ask.limit);
		return { entries, done: ask.skip + entries.length >= all.length };
	});
}

/** `count` edits, newest first, the oldest of them the birth. */
function long(count: number): LifeEntry[] {
	return Array.from({ length: count }, (_, i) =>
		entry(i + 1, i + 1, 'M', i === count - 1 ? 'born' : 'edit', `Change ${count - i}`, 1, 0, { size: count - i })
	);
}

async function settle(): Promise<void> {
	for (let i = 0; i < 8; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
}

beforeEach(() => {
	vi.clearAllMocks();
	investigate.clearState();
	serve(FILE);
	today.mockResolvedValue(TODAY);
	hunk.mockResolvedValue(HUNK);
	compare.mockResolvedValue([HUNK]);
	branch.mockResolvedValue(BRANCH_FACTS);
});

describe('reading a life', () => {
	it('asks for the first page of the target, following renames, over all time', async () => {
		investigate.show(AVATARS);
		await settle();

		expect(life).toHaveBeenCalledWith({
			target: AVATARS,
			rev: '',
			follow: true,
			since: null,
			skip: 0,
			limit: PAGE
		});
		expect(investigate.entries).toEqual(FILE);
		expect(investigate.done).toBe(true);
		expect(investigate.reading).toBe(false);
	});

	it('reads back from the revision another screen was showing', async () => {
		investigate.show({ kind: 'line', path: 'core/avatars.rs', line: 14 }, sha(9));
		await settle();

		expect(life.mock.calls[0][0].rev).toBe(sha(9));
		expect(today).toHaveBeenCalledWith('core/avatars.rs', sha(9));
	});

	/** The item: the first paint comes from the first page. */
	it('has the first page on screen before the second is back', async () => {
		const all = long(PAGE * 2 + 50);
		let release: (page: LifePage) => void = () => {};
		life.mockImplementation(async (ask: LifeAsk) => {
			if (ask.skip === 0) return { entries: all.slice(0, PAGE), done: false };
			return new Promise<LifePage>((done) => (release = done));
		});

		investigate.show(AVATARS);
		await settle();

		expect(investigate.entries).toHaveLength(PAGE);
		expect(investigate.reading).toBe(true);
		expect(investigate.done).toBe(false);
		// Something is already chosen, and today's blame already asked for.
		expect(investigate.selected).toBe(investigate.entries[0]);
		expect(today).toHaveBeenCalledTimes(1);

		release({ entries: all.slice(PAGE, PAGE * 2), done: false });
		await settle();
		expect(investigate.entries).toHaveLength(PAGE * 2);
		expect(life.mock.calls.map(([ask]) => ask.skip)).toEqual([0, PAGE, PAGE * 2]);
		// Today's blame is asked for once per life, not once per page.
		expect(today).toHaveBeenCalledTimes(1);
	});

	it('reads page after page until nothing older is left', async () => {
		serve(long(PAGE * 2 + 50));

		investigate.show(AVATARS);
		await settle();

		expect(investigate.entries).toHaveLength(PAGE * 2 + 50);
		expect(investigate.done).toBe(true);
		expect(investigate.more).toBe(false);
		expect(investigate.entries.at(-1)?.kind).toBe('born');
	});

	/** At most 5,000 commits are read, then Load more. */
	it('stops at the limit and offers to read more', async () => {
		expect(LIFE_MAX).toBe(5000);
		serve(long(LIFE_MAX + PAGE + 10));

		investigate.show(AVATARS);
		await settle();
		await settle();

		expect(investigate.entries).toHaveLength(LIFE_MAX);
		expect(investigate.more).toBe(true);
		expect(investigate.done).toBe(false);
		expect(investigate.reading).toBe(false);

		investigate.loadMore();
		await settle();

		expect(investigate.entries).toHaveLength(LIFE_MAX + PAGE + 10);
		expect(investigate.more).toBe(false);
		expect(investigate.done).toBe(true);
	});

	it('drops a page for a target that is no longer the one shown', async () => {
		let release: (page: LifePage) => void = () => {};
		life.mockImplementationOnce(() => new Promise<LifePage>((done) => (release = done)));

		investigate.show({ kind: 'file', path: 'slow.rs' });
		await settle();
		serve(FOLDER);
		investigate.show({ kind: 'folder', path: 'core' });
		await settle();
		release({ entries: FILE, done: true });
		await settle();

		expect(investigate.target).toEqual({ kind: 'folder', path: 'core' });
		expect(investigate.entries).toEqual(FOLDER);
	});

	it('says why a life could not be read', async () => {
		life.mockRejectedValueOnce('no commit nowhere');

		investigate.show(AVATARS, 'nowhere');
		await settle();

		expect(investigate.error).toContain('no commit nowhere');
		expect(investigate.reading).toBe(false);
		expect(investigate.entries).toEqual([]);
	});

	it('does nothing for the target already shown', async () => {
		investigate.show(AVATARS);
		await settle();
		life.mockClear();

		investigate.show({ ...AVATARS });

		expect(life).not.toHaveBeenCalled();
	});
});

describe('what a life is read by', () => {
	it('reads again without following renames', async () => {
		investigate.show(AVATARS);
		await settle();

		investigate.setFollow(false);
		await settle();

		expect(life.mock.calls.at(-1)?.[0].follow).toBe(false);
		expect(investigate.follow).toBe(false);
	});

	it('reads only as far back as the range', async () => {
		vi.useFakeTimers({ now: 1_800_000_000_000, toFake: ['Date'] });
		investigate.show(AVATARS);
		investigate.setRange('quarter');
		vi.useRealTimers();
		await settle();

		expect(life.mock.calls.at(-1)?.[0].since).toBe(1_800_000_000 - 90 * 86_400);

		investigate.setRange('all');
		await settle();
		expect(life.mock.calls.at(-1)?.[0].since).toBeNull();
	});

	it('sorts a branch’s pages into one order, newest first', async () => {
		// A reflog's move comes with the first page, and is older than the second's commits.
		const old = entry(9, 50, 'M', 'reset', 'reset: moving to HEAD~1', 0, 0, { path: null });
		const commits = long(PAGE + 5).map((row) => ({ ...row, kind: 'commit' as const, path: null }));
		life.mockImplementation(async (ask: LifeAsk) =>
			ask.skip === 0
				? { entries: [...commits.slice(0, PAGE), old], done: false }
				: { entries: commits.slice(PAGE), done: true }
		);

		investigate.show({ kind: 'branch', name: 'feature/avatars' });
		await settle();

		const times = investigate.entries.map((row) => row.time);
		expect(times).toEqual([...times].sort((a, b) => b - a));
		expect(investigate.entries).toHaveLength(PAGE + 6);
		expect(branch).toHaveBeenCalledWith('feature/avatars');
		expect(investigate.branch).toEqual(BRANCH_FACTS);
		expect(today).not.toHaveBeenCalled();
	});

	it('lists a folder’s structural events and biggest edits, and keeps every entry for the chart', async () => {
		serve(FOLDER);

		investigate.show({ kind: 'folder', path: 'core' });
		await settle();

		expect(investigate.entries).toHaveLength(FOLDER.length);
		expect(investigate.listed.length).toBeLessThan(FOLDER.length);
		expect(investigate.listed).toHaveLength(24);
		expect(today).not.toHaveBeenCalled();
	});
});

describe('a life is kept by its target and by where HEAD is', () => {
	it('costs nothing to come back to', async () => {
		investigate.setHead(sha(100));
		investigate.show(AVATARS);
		await settle();
		serve(BRANCH);
		investigate.show({ kind: 'branch', name: 'main' });
		await settle();
		life.mockClear();
		today.mockClear();

		investigate.show(AVATARS);
		await settle();

		expect(life).not.toHaveBeenCalled();
		expect(today).not.toHaveBeenCalled();
		expect(investigate.entries).toEqual(FILE);
		expect(investigate.today).toEqual(TODAY);
		expect(investigate.selected).toBe(investigate.entries[0]);
	});

	it('is read again once HEAD has moved', async () => {
		investigate.setHead(sha(100));
		investigate.show(AVATARS);
		await settle();
		life.mockClear();

		investigate.setHead(sha(101));
		await settle();

		expect(life).toHaveBeenCalledTimes(1);
	});

	it('is kept apart for each range and each way of following', async () => {
		investigate.show(AVATARS);
		await settle();
		life.mockClear();

		investigate.setFollow(false);
		await settle();
		investigate.setFollow(true);
		await settle();

		// Without following was new; with following was kept.
		expect(life).toHaveBeenCalledTimes(1);
	});
});

describe('the selected event', () => {
	beforeEach(async () => {
		investigate.show(AVATARS);
		await settle();
		vi.clearAllMocks();
	});

	it('reads the largest hunk of what the commit did, from the name the file had', async () => {
		investigate.select(FILE[3]);
		expect(investigate.hunk).toBeUndefined();
		await settle();

		expect(hunk).toHaveBeenCalledWith(sha(3), 'src/avatars.rs', 'src/avatar.rs');
		expect(investigate.hunk).toEqual(HUNK);
	});

	it('counts the lines today’s blame still gives the commit', () => {
		investigate.select(FILE[1]);
		expect(investigate.alive).toBe(5);

		// A commit none of whose lines survive.
		investigate.select(FILE[4]);
		expect(investigate.alive).toBe(0);
	});

	it('compares the file as the commit left it with the file today, and goes back', async () => {
		investigate.select(FILE[4]);
		await settle();

		await investigate.toggleCompare();
		expect(compare).toHaveBeenCalledWith(sha(2), 'src/avatar.rs', '', 'core/avatars.rs');
		expect(investigate.comparing).toBe(true);
		expect(investigate.compared).toEqual([HUNK]);

		await investigate.toggleCompare();
		expect(investigate.comparing).toBe(false);
		await investigate.toggleCompare();
		// Asked once: the second look is what was read.
		expect(compare).toHaveBeenCalledTimes(1);
	});

	it('goes back to the commit’s own hunk when another event is chosen', async () => {
		investigate.select(FILE[4]);
		await settle();
		await investigate.toggleCompare();

		investigate.select(FILE[1]);

		expect(investigate.comparing).toBe(false);
		expect(investigate.compared).toBeNull();
	});

	it('has no hunk for a deleted file, and asks for none', async () => {
		investigate.select(entry(9, 1, 'M', 'deleted', 'Remove it', 0, 21, { size: 0 }));
		await settle();

		expect(hunk).not.toHaveBeenCalled();
		expect(investigate.hunk).toBeNull();
	});

	it('drops a hunk for an event that is no longer the one chosen', async () => {
		let release: (value: typeof HUNK | null) => void = () => {};
		hunk.mockImplementationOnce(() => new Promise((done) => (release = done)));
		hunk.mockResolvedValueOnce(null);

		investigate.select(FILE[2]);
		investigate.select(FILE[3]);
		await settle();
		release(HUNK);
		await settle();

		expect(investigate.hunk).toBeNull();
	});
});
