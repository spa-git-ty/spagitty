// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { OpenResult, RepoInfo, Snapshot } from './types';

vi.mock('$lib/api', () => ({
	openRepo: vi.fn(),
	closeRepo: vi.fn(() => Promise.resolve()),
	snapshot: vi.fn()
}));

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

import * as api from '$lib/api';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { repo } from './repo.svelte';

const openRepo = vi.mocked(api.openRepo);
const closeRepo = vi.mocked(api.closeRepo);
const snapshot = vi.mocked(api.snapshot);
const dialog = vi.mocked(openDialog);

function info(path = '/repos/fixture', branch: string | null = 'main'): RepoInfo {
	return {
		path,
		name: path.slice(path.lastIndexOf('/') + 1),
		bare: false,
		lastFetched: null,
		head: { branch, detached: branch === null, id: 'a'.repeat(40), short: 'aaaaaaa' }
	};
}

function result(overrides: Partial<OpenResult> = {}): OpenResult {
	return {
		info: info(),
		counts: {
			commits: null,
			working: null,
			staged: null,
			conflicts: null,
			branches: 4,
			stashes: 2,
			tags: 2,
			submodules: 0
		},
		token: 3,
		...overrides
	};
}

beforeEach(async () => {
	vi.clearAllMocks();
	openRepo.mockResolvedValue(result());
	closeRepo.mockResolvedValue(undefined);
	await repo.close();
});

describe('open', () => {
	it('takes the info, counts and token from the result', async () => {
		expect(await repo.open('/repos/fixture')).toBe(true);

		expect(repo.info?.name).toBe('fixture');
		expect(repo.counts.branches).toBe(4);
		expect(repo.token).toBe(3);
		expect(repo.error).toBeNull();
	});

	it('bumps the generation, which is what makes the graph start over', async () => {
		const before = repo.generation;
		await repo.open('/repos/fixture');
		expect(repo.generation).toBe(before + 1);
	});

	it('clears busy whether it succeeds or fails', async () => {
		await repo.open('/repos/fixture');
		expect(repo.busy).toBe(false);

		openRepo.mockRejectedValueOnce('not a git repository');
		await repo.open('/tmp');
		expect(repo.busy).toBe(false);
	});

	it('reports a failure and leaves no half-open repository behind', async () => {
		await repo.open('/repos/fixture');

		openRepo.mockRejectedValueOnce('/tmp is not a git repository');
		expect(await repo.open('/tmp')).toBe(false);

		expect(repo.error).toBe('/tmp is not a git repository');
		expect(repo.info).toBeNull();
		expect(repo.token).toBeNull();
		expect(repo.counts.branches).toBeNull();
	});

	it('does not bump the generation on a failed open', async () => {
		openRepo.mockRejectedValueOnce('nope');
		const before = repo.generation;
		await repo.open('/tmp');
		expect(repo.generation).toBe(before);
	});
});

describe('choose', () => {
	it('opens the directory the user picked', async () => {
		dialog.mockResolvedValueOnce('/repos/picked');
		expect(await repo.choose()).toBe(true);
		expect(openRepo).toHaveBeenCalledWith('/repos/picked', 'date');
	});

	it('does nothing when the dialog is cancelled', async () => {
		dialog.mockResolvedValueOnce(null);
		expect(await repo.choose()).toBe(false);
		expect(openRepo).not.toHaveBeenCalled();
	});

	it('does nothing when the dialog returns several paths', async () => {
		// The dialog is configured single-select; an array would mean the
		// configuration changed under us, and opening the first one silently
		// would be a guess.
		dialog.mockResolvedValueOnce(['/a', '/b'] as unknown as string);
		expect(await repo.choose()).toBe(false);
		expect(openRepo).not.toHaveBeenCalled();
	});
});

describe('refresh', () => {
	function snap(branch: string | null): Snapshot {
		return {
			info: info('/repos/fixture', branch),
			counts: {
				commits: null,
				working: 3,
				staged: 1,
				conflicts: 0,
				branches: 5,
				stashes: 2,
				tags: 2,
				submodules: 0
			}
		};
	}

	it('does nothing when no repository is open', async () => {
		await repo.refresh();
		expect(snapshot).not.toHaveBeenCalled();
	});

	it('re-reads HEAD and the counts', async () => {
		await repo.open('/repos/fixture');
		snapshot.mockResolvedValueOnce(snap('feature/split-view'));

		await repo.refresh();

		expect(repo.info?.head.branch).toBe('feature/split-view');
		expect(repo.counts.branches).toBe(5);
	});

	it('keeps the commit count, which only the walk knows', async () => {
		await repo.open('/repos/fixture');
		repo.setCommitCount(42);
		snapshot.mockResolvedValueOnce(snap('main'));

		await repo.refresh();

		expect(repo.counts.commits).toBe(42);
	});

	it('records a failure without discarding the open repository', async () => {
		await repo.open('/repos/fixture');
		snapshot.mockRejectedValueOnce('could not read the repository');

		await repo.refresh();

		expect(repo.error).toBe('could not read the repository');
		expect(repo.info).not.toBeNull();
	});

	it('clears a previous error once a refresh succeeds', async () => {
		await repo.open('/repos/fixture');
		snapshot.mockRejectedValueOnce('transient');
		await repo.refresh();
		expect(repo.error).not.toBeNull();

		snapshot.mockResolvedValueOnce(snap('main'));
		await repo.refresh();
		expect(repo.error).toBeNull();
	});
});

describe('token and commit count', () => {
	it('takes a new token from a restarted walk', async () => {
		await repo.open('/repos/fixture');
		repo.setToken(9);
		expect(repo.token).toBe(9);
	});

	it('takes the commit count from the graph rather than estimating one', async () => {
		await repo.open('/repos/fixture');
		expect(repo.counts.commits).toBeNull();
		repo.setCommitCount(11);
		expect(repo.counts.commits).toBe(11);
	});
});

describe('close', () => {
	it('forgets the repository', async () => {
		await repo.open('/repos/fixture');
		await repo.close();

		expect(closeRepo).toHaveBeenCalled();
		expect(repo.info).toBeNull();
		expect(repo.token).toBeNull();
		expect(repo.counts.commits).toBeNull();
		expect(repo.counts.branches).toBeNull();
	});
});
