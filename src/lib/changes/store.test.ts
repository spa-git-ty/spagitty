// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { DiffSide, FileDiff, StatusEntry, WorkingCopy } from '$lib/types';

vi.mock('$lib/api', () => ({
	workingCopy: vi.fn(),
	workingDiff: vi.fn(),
	stage: vi.fn(),
	unstage: vi.fn(),
	stageHunk: vi.fn(),
	unstageHunk: vi.fn(),
	discard: vi.fn(),
	discardHunk: vi.fn(),
	commit: vi.fn(),
	headMessage: vi.fn(),
	signing: vi.fn()
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import * as api from '$lib/api';
import { calls as repoCalls, control as repoControl } from '../../testing/repo-store.svelte';
import { changes } from './store.svelte';

const workingCopy = vi.mocked(api.workingCopy);
const signing = vi.mocked(api.signing);
const workingDiff = vi.mocked(api.workingDiff);
const stage = vi.mocked(api.stage);
const unstage = vi.mocked(api.unstage);
const stageHunk = vi.mocked(api.stageHunk);
const unstageHunk = vi.mocked(api.unstageHunk);
const discard = vi.mocked(api.discard);
const discardHunk = vi.mocked(api.discardHunk);
const commit = vi.mocked(api.commit);
const headMessage = vi.mocked(api.headMessage);

function entry(path: string, status: StatusEntry['status'] = 'modified'): StatusEntry {
	return { path, status };
}

function work(overrides: Partial<WorkingCopy> = {}): WorkingCopy {
	return { staged: [], unstaged: [], conflicted: [], ...overrides };
}

function diff(path: string, hunks = 1): FileDiff {
	return {
		path,
		status: 'modified',
		binary: false,
		tooLarge: false,
		added: 1,
		removed: 1,
		hunks: Array.from({ length: hunks }, (_, i) => ({
			oldStart: i * 10 + 1,
			oldLines: 1,
			newStart: i * 10 + 1,
			newLines: 1,
			header: `@@ -${i * 10 + 1} +${i * 10 + 1} @@`,
			lines: [{ origin: 'added' as const, old: null, new: i * 10 + 1, text: 'new' }]
		}))
	};
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function deferred<T>() {
	let resolve!: (value: T) => void;
	const promise = new Promise<T>((res) => (resolve = res));
	return { promise, resolve };
}

beforeEach(() => {
	vi.clearAllMocks();
	changes.clear();
	repoControl.reset();
	workingCopy.mockResolvedValue(
		work({ staged: [entry('a.txt')], unstaged: [entry('b.txt'), entry('c.txt', 'untracked')] })
	);
	workingDiff.mockImplementation((path: string) => Promise.resolve(diff(path)));
	stage.mockResolvedValue(undefined);
	unstage.mockResolvedValue(undefined);
	stageHunk.mockResolvedValue(undefined);
	unstageHunk.mockResolvedValue(undefined);
	discard.mockResolvedValue(undefined);
	discardHunk.mockResolvedValue(undefined);
	commit.mockResolvedValue('a'.repeat(40));
	headMessage.mockResolvedValue('');
});

describe('load', () => {
	it('fills the three lists and opens the first unstaged file', async () => {
		await changes.load();
		await settle();

		expect(changes.work.staged.map((e) => e.path)).toEqual(['a.txt']);
		expect(changes.work.unstaged.map((e) => e.path)).toEqual(['b.txt', 'c.txt']);
		expect(changes.loaded).toBe(true);

		// Unstaged first: it is where the work is.
		expect(changes.selection).toEqual({ path: 'b.txt', side: 'unstaged' });
		expect(changes.file?.path).toBe('b.txt');
	});

	it('falls back to a staged file when nothing is unstaged', async () => {
		workingCopy.mockResolvedValueOnce(work({ staged: [entry('a.txt')] }));
		await changes.load();
		await settle();

		expect(changes.selection).toEqual({ path: 'a.txt', side: 'staged' });
	});

	it('selects nothing when the working copy is clean', async () => {
		workingCopy.mockResolvedValueOnce(work());
		await changes.load();
		await settle();

		expect(changes.selection).toBeNull();
		expect(changes.file).toBeNull();
		expect(workingDiff).not.toHaveBeenCalled();
	});

	it('records a failure and shows no lists', async () => {
		workingCopy.mockRejectedValueOnce('could not read the working copy');
		await changes.load();

		expect(changes.error).toBe('could not read the working copy');
		expect(changes.work.staged).toEqual([]);
		expect(changes.loading).toBe(false);
	});

	it('keeps the selection when its row survives the reload', async () => {
		await changes.load();
		await settle();
		changes.open({ path: 'c.txt', side: 'unstaged' });
		await settle();

		await changes.load();
		await settle();

		expect(changes.selection).toEqual({ path: 'c.txt', side: 'unstaged' });
	});

	it('moves the selection when its row is gone', async () => {
		await changes.load();
		await settle();

		workingCopy.mockResolvedValueOnce(work({ unstaged: [entry('z.txt')] }));
		await changes.load();
		await settle();

		expect(changes.selection).toEqual({ path: 'z.txt', side: 'unstaged' });
	});

	it('drops a slow walk that a newer one superseded', async () => {
		const slow = deferred<WorkingCopy>();
		workingCopy.mockReturnValueOnce(slow.promise);

		const first = changes.load();
		await changes.load();
		await settle();

		slow.resolve(work({ unstaged: [entry('stale.txt')] }));
		await first;
		await settle();

		expect(changes.work.unstaged.map((e) => e.path)).not.toContain('stale.txt');
	});
});

describe('open', () => {
	it('fetches the hunks for the side that was asked for', async () => {
		await changes.load();
		await settle();
		workingDiff.mockClear();

		changes.open({ path: 'a.txt', side: 'staged' });
		await settle();

		expect(workingDiff).toHaveBeenCalledWith('a.txt', 'staged');
		expect(changes.file?.path).toBe('a.txt');
	});

	it('treats the same path on the other side as a different thing to read', async () => {
		// A file can be staged in part: the two sides show different hunks.
		await changes.load();
		await settle();
		workingDiff.mockClear();

		changes.open({ path: 'b.txt', side: 'staged' });
		await settle();

		expect(workingDiff).toHaveBeenCalledWith('b.txt', 'staged');
	});

	it('does not refetch a selection that is already open', async () => {
		await changes.load();
		await settle();
		workingDiff.mockClear();

		changes.open({ path: 'b.txt', side: 'unstaged' });
		expect(workingDiff).not.toHaveBeenCalled();
	});

	it('retries a selected file whose last fetch left nothing to show', async () => {
		await changes.load();
		await settle();

		workingDiff.mockRejectedValueOnce('no file b.txt');
		changes.open({ path: 'b.txt', side: 'unstaged' }, true);
		await settle();
		expect(changes.file).toBeNull();

		workingDiff.mockClear();
		changes.open({ path: 'b.txt', side: 'unstaged' });
		await settle();

		expect(workingDiff).toHaveBeenCalledWith('b.txt', 'unstaged');
		expect(changes.file?.path).toBe('b.txt');
	});

	it('records a file error without disturbing the lists', async () => {
		await changes.load();
		await settle();

		workingDiff.mockRejectedValueOnce('no file z.txt');
		changes.open({ path: 'z.txt', side: 'unstaged' });
		await settle();

		expect(changes.fileError).toBe('no file z.txt');
		expect(changes.fileLoading).toBe(false);
		expect(changes.work.unstaged).toHaveLength(2);
	});

	it('drops a slow file load that a newer selection superseded', async () => {
		await changes.load();
		await settle();

		const slow = deferred<FileDiff>();
		workingDiff.mockReturnValueOnce(slow.promise);

		changes.open({ path: 'c.txt', side: 'unstaged' });
		changes.open({ path: 'a.txt', side: 'staged' });
		await settle();

		slow.resolve(diff('c.txt'));
		await settle();

		expect(changes.file?.path).toBe('a.txt');
	});
});

describe('staging', () => {
	it('stages the paths it is given and re-reads afterwards', async () => {
		await changes.load();
		await settle();
		workingCopy.mockClear();

		expect(await changes.stage(['b.txt'])).toBe(true);

		expect(stage).toHaveBeenCalledWith(['b.txt']);
		// Staging changes what every other row means, so the lists are re-read
		// rather than patched in place.
		expect(workingCopy).toHaveBeenCalled();
		expect(repoCalls.refreshed).toBeGreaterThan(0);
	});

	it('unstages the paths it is given', async () => {
		await changes.load();
		await settle();

		expect(await changes.unstage(['a.txt'])).toBe(true);
		expect(unstage).toHaveBeenCalledWith(['a.txt']);
	});

	it('records a write failure and still re-reads', async () => {
		await changes.load();
		await settle();
		workingCopy.mockClear();

		stage.mockRejectedValueOnce('could not stage');
		expect(await changes.stage(['b.txt'])).toBe(false);

		expect(changes.writeError).toBe('could not stage');
		expect(workingCopy).toHaveBeenCalled();
	});

	it('refuses a second write while one is in flight', async () => {
		await changes.load();
		await settle();

		const slow = deferred<void>();
		stage.mockReturnValueOnce(slow.promise);

		const first = changes.stage(['b.txt']);
		const second = await changes.stage(['c.txt']);

		expect(second).toBe(false);
		expect(stage).toHaveBeenCalledTimes(1);

		slow.resolve();
		await first;
	});
});

describe('hunks', () => {
	it('stages a hunk when the open file is unstaged', async () => {
		await changes.load();
		await settle();

		await changes.hunk(1, '@@ -11 +11 @@');

		expect(stageHunk).toHaveBeenCalledWith('b.txt', 1, '@@ -11 +11 @@');
		expect(unstageHunk).not.toHaveBeenCalled();
	});

	it('unstages a hunk when the open file is staged', async () => {
		// The same button, the opposite operation — which side is open decides.
		await changes.load();
		await settle();
		changes.open({ path: 'a.txt', side: 'staged' });
		await settle();

		await changes.hunk(0, '@@ -1 +1 @@');

		expect(unstageHunk).toHaveBeenCalledWith('a.txt', 0, '@@ -1 +1 @@');
		expect(stageHunk).not.toHaveBeenCalled();
	});

	it('does nothing with no file open', async () => {
		expect(await changes.hunk(0, '@@ -1 +1 @@')).toBe(false);
		expect(stageHunk).not.toHaveBeenCalled();
	});

	it('surfaces a refusal rather than pretending it worked', async () => {
		await changes.load();
		await settle();

		stageHunk.mockRejectedValueOnce('b.txt changed since it was read; reload and try again');
		expect(await changes.hunk(0, '@@ -1 +1 @@')).toBe(false);

		expect(changes.writeError).toContain('changed since it was read');
	});
});

describe('committing', () => {
	it('needs a subject', async () => {
		await changes.load();
		await settle();

		expect(changes.canCommit).toBe(false);
		changes.setSubject('   ');
		expect(changes.canCommit).toBe(false);

		changes.setSubject('A subject');
		expect(changes.canCommit).toBe(true);
	});

	it('needs something staged', async () => {
		workingCopy.mockResolvedValueOnce(work({ unstaged: [entry('b.txt')] }));
		await changes.load();
		await settle();
		changes.setSubject('A subject');

		expect(changes.canCommit).toBe(false);
	});

	it('refuses while anything is conflicted', async () => {
		workingCopy.mockResolvedValueOnce(
			work({ staged: [entry('a.txt')], conflicted: [entry('shared.txt')] })
		);
		await changes.load();
		await settle();
		changes.setSubject('A subject');

		expect(changes.canCommit).toBe(false);
	});

	it('allows an amend with nothing staged, since it can reword', async () => {
		workingCopy.mockResolvedValueOnce(work());
		await changes.load();
		await settle();
		changes.setSubject('Reworded');
		await changes.setAmend(true);

		expect(changes.canCommit).toBe(true);
	});

	it('sends the message and clears it afterwards', async () => {
		await changes.load();
		await settle();
		changes.setSubject('A subject');
		changes.setBody('A body.');

		expect(await changes.commit()).toBe(true);

		expect(commit).toHaveBeenCalledWith('A subject', 'A body.', false);
		expect(changes.subject).toBe('');
		expect(changes.body).toBe('');
		expect(changes.amend).toBe(false);
	});

	it('keeps the message when the commit fails', async () => {
		// Retyping a message because a hook rejected it is the wrong punishment.
		await changes.load();
		await settle();
		changes.setSubject('Blocked by a hook');
		commit.mockRejectedValueOnce('git commit failed: the hook says no');

		expect(await changes.commit()).toBe(false);

		expect(changes.subject).toBe('Blocked by a hook');
		expect(changes.writeError).toContain('the hook says no');
	});

	it('does nothing when it cannot commit', async () => {
		await changes.load();
		await settle();

		expect(await changes.commit()).toBe(false);
		expect(commit).not.toHaveBeenCalled();
	});
});

describe('amend', () => {
	it('offers the previous message when there is nothing typed', async () => {
		headMessage.mockResolvedValueOnce('Previous subject\n\nPrevious body.');
		await changes.setAmend(true);

		expect(changes.subject).toBe('Previous subject');
		expect(changes.body).toBe('Previous body.');
	});

	it('does not overwrite a message already being written', async () => {
		changes.setSubject('Mine');

		await changes.setAmend(true);

		expect(changes.subject).toBe('Mine');
		// It does not even ask: there is nothing it could do with the answer.
		expect(headMessage).not.toHaveBeenCalled();
	});

	it('leaves the boxes alone when there is no previous commit', async () => {
		headMessage.mockResolvedValueOnce('');
		await changes.setAmend(true);

		expect(changes.subject).toBe('');
		expect(changes.amend).toBe(true);
	});

	it('does not fetch a message when switching amend off', async () => {
		await changes.setAmend(false);
		expect(headMessage).not.toHaveBeenCalled();
	});

	it('survives the previous message being unreadable', async () => {
		headMessage.mockRejectedValueOnce('no repository is open');
		await expect(changes.setAmend(true)).resolves.toBeUndefined();
		expect(changes.amend).toBe(true);
	});
});

describe('clear', () => {
	it('forgets everything, including a message in progress', async () => {
		await changes.load();
		await settle();
		changes.setSubject('Half-written');

		changes.clear();

		expect(changes.loaded).toBe(false);
		expect(changes.work.staged).toEqual([]);
		expect(changes.selection).toBeNull();
		expect(changes.file).toBeNull();
		expect(changes.subject).toBe('');
		expect(changes.writeError).toBeNull();
	});

	it('stops an in-flight walk from landing afterwards', async () => {
		const slow = deferred<WorkingCopy>();
		workingCopy.mockReturnValueOnce(slow.promise);

		const pending = changes.load();
		changes.clear();
		slow.resolve(work({ unstaged: [entry('b.txt')] }));
		await pending;
		await settle();

		expect(changes.loaded).toBe(false);
	});
});

describe('the side a file is opened on', () => {
	it('is the side the row was in', async () => {
		const sides: DiffSide[] = ['staged', 'unstaged'];
		for (const side of sides) {
			changes.clear();
			workingCopy.mockResolvedValueOnce(
				work(side === 'staged' ? { staged: [entry('x.txt')] } : { unstaged: [entry('x.txt')] })
			);
			await changes.load();
			await settle();

			expect(changes.selection?.side).toBe(side);
		}
	});
});

describe('discard', () => {
	it('throws away the paths it is given and re-reads afterwards', async () => {
		await changes.load();
		await settle();
		workingCopy.mockClear();

		expect(await changes.discard(['b.txt', 'c.txt'])).toBe(true);

		expect(discard).toHaveBeenCalledWith(['b.txt', 'c.txt']);
		// Discarding changes what every other row means, the same as staging.
		expect(workingCopy).toHaveBeenCalled();
		expect(repoCalls.refreshed).toBeGreaterThan(0);
	});

	it('asks nothing itself — the confirmation is the caller’s', async () => {
		// The store is the layer the tests can drive without a dialog on
		// screen; `discard.ts` is where the question lives.
		expect(await changes.discard(['b.txt'])).toBe(true);
		expect(discard).toHaveBeenCalledTimes(1);
	});

	it('surfaces a refusal rather than pretending it worked', async () => {
		discard.mockRejectedValueOnce(new Error('permission denied'));

		expect(await changes.discard(['b.txt'])).toBe(false);
		expect(changes.writeError).toContain('permission denied');
	});

	it('discards a hunk of the open unstaged file', async () => {
		await changes.load();
		await settle();

		expect(await changes.discardHunk(1, '@@ -11 +11 @@')).toBe(true);
		expect(discardHunk).toHaveBeenCalledWith('b.txt', 1, '@@ -11 +11 @@');
	});

	it('refuses to discard a hunk from the staged side', async () => {
		// There is nothing to throw away there: the change has been kept once
		// already, and unstaging is what brings it back within reach.
		await changes.load();
		await settle();
		changes.open({ path: 'a.txt', side: 'staged' });
		await settle();

		expect(await changes.discardHunk(0, '@@ -1 +1 @@')).toBe(false);
		expect(discardHunk).not.toHaveBeenCalled();
	});

	it('does nothing with no file open', async () => {
		expect(await changes.discardHunk(0, '@@ -1 +1 @@')).toBe(false);
		expect(discardHunk).not.toHaveBeenCalled();
	});
});
