// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { file, side, sides, state } from '../../testing/git-fixtures';

vi.mock('$lib/api', () => ({
	conflicts: vi.fn(),
	conflictSides: vi.fn(),
	conflictSettle: vi.fn(),
	conflictContinue: vi.fn(),
	conflictAbort: vi.fn()
}));

import * as api from '$lib/api';
import { conflicts, fileFrom } from './store.svelte';

const listCall = vi.mocked(api.conflicts);
const sidesCall = vi.mocked(api.conflictSides);
const settleCall = vi.mocked(api.conflictSettle);
const continueCall = vi.mocked(api.conflictContinue);
const abortCall = vi.mocked(api.conflictAbort);

beforeEach(() => {
	vi.clearAllMocks();
	conflicts.clear();
	listCall.mockResolvedValue(state());
	sidesCall.mockImplementation(async (path) => sides(path));
	settleCall.mockResolvedValue(undefined);
	continueCall.mockResolvedValue(undefined);
	abortCall.mockResolvedValue(undefined);
});

describe('reading', () => {
	it('reads every conflicted file whole, ours as A and theirs as B', async () => {
		await conflicts.load();

		expect(conflicts.operation).toBe('merge');
		expect(conflicts.operationLabel).toBe('merge');
		expect(conflicts.read).toHaveLength(1);
		const [shared] = conflicts.read;
		expect(shared.regions[0]).toMatchObject({ a: ['OURS'], b: ['THEIRS'], aLine: 2, bLine: 2 });
		expect(conflicts.choices['shared.txt']).toEqual([null]);
	});

	it('a file deleted on one side is chosen whole', () => {
		const read = fileFrom(sides('gone.txt', { kind: 'deletedByThem', theirs: null, merged: side('one\nOURS\nthree\n') }));
		expect(read.whole).toBe(true);
		expect(read.bExists).toBe(false);
	});

	it('says what went wrong, and empties the list', async () => {
		listCall.mockRejectedValueOnce('index unavailable');
		await conflicts.load();
		expect(conflicts.error).toBe('index unavailable');
		expect(conflicts.read).toEqual([]);
	});

	it('keeps a choice across a reload while the file is the same, and drops it once it is not', async () => {
		await conflicts.load();
		conflicts.choose('shared.txt', 0, { mode: 'b' });
		await conflicts.load();
		expect(conflicts.choices['shared.txt']).toEqual([{ mode: 'b' }]);

		sidesCall.mockImplementation(async (path) =>
			sides(path, { merged: side('one\n<<<<<<< HEAD\nOURS, again\n=======\nTHEIRS\n>>>>>>> theirs\nthree\n') })
		);
		await conflicts.load();
		expect(conflicts.choices['shared.txt']).toEqual([null]);
	});
});

describe('marking a file resolved', () => {
	it('waits for every region, then writes the result and stages it', async () => {
		await conflicts.load();
		expect(conflicts.settleable('shared.txt')).toBe(false);
		expect(await conflicts.settle('shared.txt')).toBe(false);
		expect(settleCall).not.toHaveBeenCalled();

		conflicts.choose('shared.txt', 0, { mode: 'ba' });
		expect(conflicts.settleable('shared.txt')).toBe(true);
		listCall.mockResolvedValue(state({ files: [] }));

		expect(await conflicts.settle('shared.txt')).toBe(true);
		expect(settleCall).toHaveBeenCalledWith('shared.txt', 'one\nTHEIRS\nOURS\nthree\n', null);
		expect(conflicts.allResolved).toBe(true);
	});

	it('takes a whole side for a file that conflicts as a whole', async () => {
		listCall.mockResolvedValue(state({ files: [file('gone.txt', 'deletedByThem')] }));
		sidesCall.mockResolvedValue(sides('gone.txt', { kind: 'deletedByThem', theirs: null, merged: side('one\nOURS\nthree\n') }));
		await conflicts.load();

		conflicts.whole('gone.txt', 'b');
		await conflicts.settle('gone.txt');

		expect(settleCall).toHaveBeenCalledWith('gone.txt', null, 'theirs');
	});

	it('a file whose markers are gone from disk can be marked resolved as it is', async () => {
		sidesCall.mockResolvedValue(sides('shared.txt', { merged: side('one\nfixed by hand\nthree\n') }));
		await conflicts.load();

		expect(conflicts.asOnDisk('shared.txt')).toBe(true);
		await conflicts.settle('shared.txt');

		expect(settleCall).toHaveBeenCalledWith('shared.txt', 'one\nfixed by hand\nthree\n', null);
	});

	it('reports a refusal and reads again', async () => {
		await conflicts.load();
		conflicts.choose('shared.txt', 0, { mode: 'a' });
		settleCall.mockRejectedValueOnce('git add failed');

		expect(await conflicts.settle('shared.txt')).toBe(false);
		expect(conflicts.writeError).toBe('git add failed');
		expect(listCall).toHaveBeenCalledTimes(2);
	});
});

describe('the operation', () => {
	it('continues and aborts through git', async () => {
		await conflicts.load();
		expect(await conflicts.continue()).toBe(true);
		expect(continueCall).toHaveBeenCalled();
		conflicts.choose('shared.txt', 0, { mode: 'a' });
		expect(await conflicts.abort()).toBe(true);
		expect(abortCall).toHaveBeenCalled();
	});
});
