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

/** The network store, as far as a push from Merger sees it. */
const net = vi.hoisted(() => ({
	running: false,
	error: null as string | null,
	push: vi.fn(),
	whenDone: vi.fn()
}));
vi.mock('$lib/network/store.svelte', () => ({
	network: {
		get running() {
			return net.running;
		},
		get error() {
			return net.error;
		},
		push: net.push,
		whenDone: net.whenDone
	}
}));

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
	net.running = false;
	net.error = null;
	net.push.mockResolvedValue(true);
	net.whenDone.mockResolvedValue(true);
});

/** Pull request #61 resolved here on `pr-61`, with the branches a fetch left behind. */
async function resolved(rows = [branchRow('origin/main', { kind: 'remote' }), branchRow('origin/feat/thing', { kind: 'remote' })]) {
	vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h1', base: 'b1', mergeBase: 'm0' });
	vi.mocked(api.reviewCheckOut).mockResolvedValue({ branch: 'pr-61', upstream: null, renamed: true });
	vi.mocked(api.branches).mockResolvedValue([branchRow('pr-61', { current: true }), branchRow('main'), ...rows]);
	vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
	merger.presentPullRequest(PR);
	await merger.resolvePullRequestLocally();
}

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

	it('pushes the resolved branch to the pull request’s own branch, on the remote it is on', async () => {
		await resolved();
		expect(merger.origin).toMatchObject({ local: { branch: 'pr-61', target: 'origin/main', remote: 'origin' } });

		expect(await merger.pushPullRequestHead()).toBe(true);

		// A named remote, or git is never given the refspec; and through the
		// network store, which lets go of the worker when the push ends (BUG-076).
		expect(net.push).toHaveBeenCalledWith('origin', 'pr-61:feat/thing');
		expect(api.push).not.toHaveBeenCalled();
		expect(merger.pushed).toBe(true);
		expect(merger.pushError).toBeNull();
	});

	it('says Pushed only once git has said so', async () => {
		await resolved();
		let end: (ok: boolean) => void = () => {};
		net.whenDone.mockReturnValue(new Promise<boolean>((resolve) => (end = resolve)));

		const pushing = merger.pushPullRequestHead();
		await vi.waitFor(() => expect(net.whenDone).toHaveBeenCalled());
		expect(merger.pushing).toBe(true);
		expect(merger.pushed).toBe(false);

		end(true);
		expect(await pushing).toBe(true);
		expect(merger.pushed).toBe(true);
		expect(merger.pushing).toBe(false);
	});

	it('shows git’s own words when the push is rejected, and is not marked pushed (BUG-076)', async () => {
		await resolved();
		net.whenDone.mockImplementation(async () => {
			net.error = '! [rejected] pr-61 -> feat/thing (fetch first)';
			return false;
		});

		expect(await merger.pushPullRequestHead()).toBe(false);

		expect(merger.pushed).toBe(false);
		expect(merger.pushError).toContain('rejected');
		expect(merger.pushing).toBe(false);
	});

	it('says why when the push could not start', async () => {
		await resolved();
		net.push.mockImplementation(async () => {
			net.error = 'no repository is open';
			return false;
		});

		expect(await merger.pushPullRequestHead()).toBe(false);

		expect(net.whenDone).not.toHaveBeenCalled();
		expect(merger.pushError).toContain('no repository is open');
		expect(merger.pushed).toBe(false);
	});

	it('waits its turn when a fetch or a push is already running', async () => {
		await resolved();
		net.running = true;

		expect(await merger.pushPullRequestHead()).toBe(false);

		expect(net.push).not.toHaveBeenCalled();
		expect(merger.pushError).toContain('is running');
	});

	it('does not push a branch the remote is not seen to have: a fork’s is not there (BUG-076)', async () => {
		await resolved([branchRow('origin/main', { kind: 'remote' })]);
		expect(merger.origin).toMatchObject({ local: { branch: 'pr-61', remote: null } });

		expect(await merger.pushPullRequestHead()).toBe(false);

		expect(net.push).not.toHaveBeenCalled();
		expect(merger.pushError).toContain('feat/thing');
		expect(merger.pushed).toBe(false);
	});

	it('does not push when it was not resolved here', async () => {
		merger.presentPullRequest(PR);
		expect(await merger.pushPullRequestHead()).toBe(false);
		expect(net.push).not.toHaveBeenCalled();
	});
});

describe('a Farm task', () => {
	const TASK = { id: 'T3', branch: 'feat/tab-drag', target: 'main', title: 'The work' };

	it('is merged like any two branches, with its target receiving it', () => {
		merger.presentFarmTask(TASK);
		expect(merger.origin).toMatchObject({ kind: 'farmTask', id: 'T3' });
	});

	it('keeps its commits and its direction, so the Farm can see it is merged (BUG-077)', async () => {
		vi.mocked(api.branches).mockResolvedValue([branchRow('main', { current: true }), branchRow(TASK.branch)]);
		vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast());
		merger.presentFarmTask(TASK);
		await merger.prime('/repo', 'main');

		// A squash or a rebase writes other commits than the task's own.
		const refused = merger.choices.filter((choice) => choice.id === 'squash' || choice.id === 'rebase');
		expect(refused).toHaveLength(2);
		for (const choice of refused) expect(choice.reason).toContain('Farm');
		merger.setStrategy('squash');
		expect(merger.strategy).not.toBe('squash');

		merger.setInto('b');
		merger.swap();
		expect(merger.into).toBe('a');
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
