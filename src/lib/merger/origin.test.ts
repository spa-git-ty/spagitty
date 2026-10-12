// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What Merger is opened for (FEAT-117): every merge is made here — a pull
 * request through its host with the host's methods, its conflicts resolved
 * here first, and a Farm task like any two branches.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { branchRow, cleanForecast, forecast } from '../../testing/merger-fixtures';
import { request } from '../../testing/git-fixtures';

vi.mock('$lib/api');
vi.mock('$lib/farm/api');

import * as api from '$lib/api';
import * as farmApi from '$lib/farm/api';
import { requests } from '$lib/requests/store.svelte';
import { merger } from './store.svelte';

const PR = request({ number: 61, title: 'Fix the thing', sourceBranch: 'feat/thing', targetBranch: 'main' });

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	requests.clear();
	localStorage.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.tags).mockResolvedValue([]);
	vi.mocked(api.forgeRepo).mockResolvedValue(null);
});

describe('a pull request', () => {
	it('is merged through the host with the method, title and message chosen', async () => {
		vi.mocked(api.mergePullRequest).mockResolvedValue(undefined);
		merger.presentPullRequest(PR);
		merger.setPrMethod('squash');
		merger.setPrTitle(' Fix it ');
		merger.setPrMessage('All of it.');

		expect(await merger.mergePullRequest()).toBe(true);

		expect(api.mergePullRequest).toHaveBeenCalledWith(61, 'squash', 'Fix it', 'All of it.');
		expect(merger.prMerged).toBe(true);
		expect(merger.prError).toBeNull();
	});

	it('sends no title or message when it is rebased, which writes no commit', async () => {
		vi.mocked(api.mergePullRequest).mockResolvedValue(undefined);
		merger.presentPullRequest(PR);
		merger.setPrMethod('rebase');
		merger.setPrTitle('ignored');

		await merger.mergePullRequest();

		expect(api.mergePullRequest).toHaveBeenCalledWith(61, 'rebase', undefined, undefined);
	});

	it('shows the host’s own sentence when it refuses, and is not marked merged', async () => {
		vi.mocked(api.mergePullRequest).mockRejectedValue('github.com: Pull Request is not mergeable');
		merger.presentPullRequest(PR);

		expect(await merger.mergePullRequest()).toBe(false);

		expect(merger.prError).toContain('not mergeable');
		expect(merger.prMerged).toBe(false);
		expect(merger.prMerging).toBe(false);
	});

	it('does nothing without a pull request to merge', async () => {
		expect(await merger.mergePullRequest()).toBe(false);
		expect(api.mergePullRequest).not.toHaveBeenCalled();
	});

	it('is resolved here, on its own branch, with its target coming in', async () => {
		vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h1', base: 'b1', mergeBase: 'm0' });
		vi.mocked(api.reviewCheckOut).mockResolvedValue({ branch: 'feat/thing', upstream: null, renamed: false });
		vi.mocked(api.branches).mockResolvedValue([
			branchRow('feat/thing', { current: true, id: 'h1' }),
			branchRow('main', { id: 'old' }),
			branchRow('origin/main', { kind: 'remote', id: 'b1' })
		]);
		vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
		merger.presentPullRequest(request({ number: 61, headSha: 'h1', sourceBranch: 'feat/thing', targetBranch: 'main' }));

		expect(await merger.resolvePullRequestLocally()).toBe(true);

		expect(api.reviewCheckout).toHaveBeenCalledWith(61, 'main', 'h1');
		// The fetched, remote-tracking target, not a stale local one.
		expect(merger.a).toBe('feat/thing');
		expect(merger.b).toBe('origin/main');
		expect(merger.into).toBe('a');
		expect(merger.origin).toMatchObject({ kind: 'pullRequest', local: { branch: 'feat/thing', target: 'origin/main' } });
	});

	it('says why it could not be fetched and stays as it was', async () => {
		vi.mocked(api.reviewCheckout).mockRejectedValue('no remote to fetch from');
		merger.presentPullRequest(PR);

		expect(await merger.resolvePullRequestLocally()).toBe(false);

		expect(merger.prepareError).toContain('no remote');
		expect(merger.origin).toMatchObject({ kind: 'pullRequest', local: null });
		expect(merger.preparing).toBe(false);
	});

	it('pushes the resolved branch to the pull request’s own branch', async () => {
		vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h1', base: 'b1', mergeBase: 'm0' });
		vi.mocked(api.reviewCheckOut).mockResolvedValue({ branch: 'pr-61', upstream: null, renamed: true });
		vi.mocked(api.branches).mockResolvedValue([branchRow('pr-61', { current: true }), branchRow('main')]);
		vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
		vi.mocked(api.push).mockResolvedValue(0);
		merger.presentPullRequest(PR);
		await merger.resolvePullRequestLocally();

		expect(await merger.pushPullRequestHead()).toBe(true);

		expect(api.push).toHaveBeenCalledWith('', 'pr-61:feat/thing');
		expect(merger.pushed).toBe(true);
	});

	it('does not push when it was not resolved here', async () => {
		merger.presentPullRequest(PR);
		expect(await merger.pushPullRequestHead()).toBe(false);
		expect(api.push).not.toHaveBeenCalled();
	});
});

describe('a Farm task', () => {
	const TASK = { id: 'T3', branch: 'feat/tab-drag', target: 'main', title: 'The work' };

	it('is merged like any two branches, with its target receiving it', () => {
		merger.presentFarmTask(TASK);
		expect(merger.origin).toMatchObject({ kind: 'farmTask', id: 'T3' });
	});

	it('is told to the Farm once the merge is written', async () => {
		vi.mocked(api.branches).mockResolvedValue([branchRow('main', { current: true }), branchRow(TASK.branch)]);
		vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast());
		vi.mocked(api.mergerLand).mockResolvedValue({ target: 'main', commit: 'c1', short: 'c1', written: 1 } as never);
		vi.mocked(farmApi.taskMerged).mockResolvedValue(undefined);
		merger.presentFarmTask(TASK);
		await merger.prime('/repo', 'main');

		expect(await merger.land()).toBe(true);

		expect(farmApi.taskMerged).toHaveBeenCalledWith('T3');
		expect(merger.farmError).toBeNull();
	});

	it('keeps the merge and says so when the Farm could not be told', async () => {
		vi.mocked(api.branches).mockResolvedValue([branchRow('main', { current: true }), branchRow(TASK.branch)]);
		vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast());
		vi.mocked(api.mergerLand).mockResolvedValue({ target: 'main', commit: 'c1', short: 'c1', written: 1 } as never);
		vi.mocked(farmApi.taskMerged).mockRejectedValue('farm-branch is not in main yet');
		merger.presentFarmTask(TASK);
		await merger.prime('/repo', 'main');

		expect(await merger.land()).toBe(true);

		expect(merger.phase).toBe('done');
		expect(merger.farmError).toContain('not in main yet');
	});

	it('is not told to the Farm for a merge that was not made for it', async () => {
		vi.mocked(api.branches).mockResolvedValue([branchRow('main', { current: true }), branchRow('feat/tab-drag')]);
		vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast());
		vi.mocked(api.mergerLand).mockResolvedValue({ target: 'main', commit: 'c1', short: 'c1', written: 1 } as never);
		merger.present({ a: 'main', b: 'feat/tab-drag', into: 'a' });
		await merger.prime('/repo', 'main');

		await merger.land();

		expect(farmApi.taskMerged).not.toHaveBeenCalled();
		expect(merger.origin).toBeNull();
	});
});
