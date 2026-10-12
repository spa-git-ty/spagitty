// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * After a merge has landed (FEAT-123): the branch that received it is checked
 * out, and a push of it is offered.
 *
 * The merge is made before any of this starts, and nothing here may make it
 * look otherwise: a checkout git refuses, or a push the remote rejects, is
 * said beside a merge that still landed.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { branchRow, cleanForecast, side } from '../../testing/merger-fixtures';

vi.mock('$lib/api');
vi.mock('$lib/farm/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

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
import { activity } from '$lib/activity/store.svelte';
import { calls as repoCalls, control as repoControl } from '../../testing/repo-store.svelte';
import { request } from '../../testing/git-fixtures';
import type { Remote } from '$lib/types';
import { merger } from './store.svelte';

const LANDED = { target: 'main', commit: '4f2c9d1'.padEnd(40, '0'), short: '4f2c9d1', written: 1 };
const ORIGIN: Remote = {
	name: 'origin',
	url: 'git@example.com:spa/spagitty.git',
	pushUrl: null,
	host: 'generic',
	refs: 4
};

/** `main` and `feat/tab-drag`, with `checkedOut` checked out here. */
async function planned(checkedOut: string | null = 'feat/tab-drag', forecast = cleanForecast()) {
	vi.mocked(api.mergerForecast).mockResolvedValue(forecast);
	merger.present({ a: 'main', b: 'feat/tab-drag', into: 'a' });
	await merger.prime('/repos/fixture', checkedOut);
}

/** The forecast with `main` checked out nowhere: the ordinary case to switch to. */
const free = () => cleanForecast({ a: side({ checkedOut: null }) });

/** Land, and wait for what follows it. */
async function land() {
	expect(await merger.land()).toBe(true);
	await vi.waitFor(() => expect(merger.checkout?.state).not.toBe('switching'));
	await vi.waitFor(() => expect(api.remotes).toHaveBeenCalled());
	await Promise.resolve();
}

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	activity.reset();
	repoControl.reset();
	localStorage.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.tags).mockResolvedValue([]);
	vi.mocked(api.branches).mockResolvedValue([
		branchRow('main', { upstream: 'origin/main' }),
		branchRow('feat/tab-drag', { current: true }),
		branchRow('origin/main', { kind: 'remote', fullName: 'refs/remotes/origin/main' })
	]);
	vi.mocked(api.remotes).mockResolvedValue([ORIGIN]);
	vi.mocked(api.mergerLand).mockResolvedValue(LANDED);
	vi.mocked(api.checkout).mockResolvedValue(undefined);
	net.running = false;
	net.error = null;
	net.push.mockResolvedValue(true);
	net.whenDone.mockResolvedValue(true);
});

describe('checking out the branch that received the merge', () => {
	it('checks it out once the merge has landed, and not before', async () => {
		await planned('feat/tab-drag', free());
		const order: string[] = [];
		vi.mocked(api.mergerLand).mockImplementation(async () => {
			order.push('land');
			return LANDED;
		});
		vi.mocked(api.checkout).mockImplementation(async () => void order.push('checkout'));

		await land();

		expect(order).toEqual(['land', 'checkout']);
		expect(api.checkout).toHaveBeenCalledWith('main');
		expect(merger.checkout).toEqual({ state: 'done' });
		expect(merger.current).toBe('main');
	});

	it('tells the rest of the window that HEAD moved', async () => {
		await planned('feat/tab-drag', free());
		const before = repoCalls.refreshed;

		await land();

		expect(repoCalls.refreshed).toBeGreaterThan(before);
	});

	it('says what it is doing while git does it (FEAT-121)', async () => {
		await planned('feat/tab-drag', free());
		let finish!: () => void;
		vi.mocked(api.checkout).mockReturnValue(new Promise<void>((resolve) => (finish = resolve)));

		expect(await merger.land()).toBe(true);
		await vi.waitFor(() => expect(merger.checkout).toEqual({ state: 'switching' }));
		expect(activity.label).toBe('Switching to main…');
		// The merge is already made and said to be: the dialog does not wait.
		expect(merger.phase).toBe('done');

		finish();
		await vi.waitFor(() => expect(merger.checkout).toEqual({ state: 'done' }));
		expect(activity.running).toBe(false);
	});

	it('checks nothing out when it is already the branch checked out', async () => {
		await planned('main');

		await land();

		expect(api.checkout).not.toHaveBeenCalled();
		expect(merger.checkout).toEqual({ state: 'already' });
	});

	it('checks out a new branch the merge was made into', async () => {
		await planned('feat/tab-drag');
		merger.setInto('new');
		vi.mocked(api.mergerLand).mockResolvedValue({ ...LANDED, target: 'merge/main-tab-drag' });

		await land();

		expect(api.checkout).toHaveBeenCalledWith('merge/main-tab-drag');
		expect(merger.checkout).toEqual({ state: 'done' });
	});

	it('checks out the other branch when the merge went the other way', async () => {
		await planned('main');
		merger.setInto('b');
		vi.mocked(api.mergerLand).mockResolvedValue({ ...LANDED, target: 'feat/tab-drag' });

		await land();

		expect(api.checkout).toHaveBeenCalledWith('feat/tab-drag');
	});

	it('leaves it where it is when another worktree has it checked out', async () => {
		// git allows a branch to be checked out once. Landing brought it
		// forward there; a checkout here could only fail.
		await planned('feat/tab-drag', cleanForecast({ a: side({ checkedOut: '/repos/fixture-main' }) }));

		await land();

		expect(api.checkout).not.toHaveBeenCalled();
		expect(merger.checkout).toEqual({ state: 'elsewhere', path: '/repos/fixture-main' });
	});

	it('knows this worktree by its path, however the path is spelt', async () => {
		// HEAD was not read as a branch, and the forecast says where it is.
		await planned(null, cleanForecast({ a: side({ checkedOut: 'C:\\repos\\Fixture\\' }) }));
		vi.mocked(api.mergerForecast).mockClear();
		await merger.prime('c:/repos/fixture', null);

		await land();

		expect(api.checkout).not.toHaveBeenCalled();
		expect(merger.checkout).toEqual({ state: 'already' });
	});

	it("keeps the merge and shows git's words when the checkout is refused", async () => {
		await planned('feat/tab-drag', free());
		vi.mocked(api.checkout).mockRejectedValue(
			'error: Your local changes to the following files would be overwritten by checkout'
		);

		await land();

		expect(merger.phase).toBe('done');
		expect(merger.landed?.landed.target).toBe('main');
		expect(merger.landError).toBeNull();
		expect(merger.checkout).toMatchObject({ state: 'failed' });
		expect(merger.checkout).toMatchObject({ error: expect.stringContaining('would be overwritten') });
		expect(merger.current).toBe('feat/tab-drag');
	});

	it('can be asked for again after a refusal', async () => {
		await planned('feat/tab-drag', free());
		vi.mocked(api.checkout).mockRejectedValueOnce('would be overwritten');
		await land();

		expect(await merger.checkOutLanded()).toBe(true);

		expect(merger.checkout).toEqual({ state: 'done' });
		expect(api.checkout).toHaveBeenCalledTimes(2);
	});

	it('waits its turn rather than check out over a write still running', async () => {
		await planned('feat/tab-drag', free());
		vi.mocked(api.checkout).mockRejectedValueOnce('would be overwritten');
		await land();
		// The way out of that refusal is a stash, and it is still going when
		// the checkout is asked for again.
		activity.begin('stash', 'Stashing your changes');

		expect(await merger.checkOutLanded()).toBe(false);

		expect(api.checkout).toHaveBeenCalledTimes(1);
		expect(merger.checkout).toEqual({ state: 'failed', error: 'Stashing your changes is still running' });
	});

	it('is not done for a Farm task, which goes where the Farm meant it to', async () => {
		vi.mocked(farmApi.taskMerged).mockResolvedValue(undefined);
		vi.mocked(api.mergerForecast).mockResolvedValue(free());
		merger.presentFarmTask({ id: 'T3', branch: 'feat/tab-drag', target: 'main', title: 'The work' });
		await merger.prime('/repos/fixture', 'feat/tab-drag');

		expect(await merger.land()).toBe(true);
		await Promise.resolve();

		expect(api.checkout).not.toHaveBeenCalled();
		expect(merger.checkout).toBeNull();
		expect(merger.pushTo).toBeNull();
	});

	it('does not land over a write still running, and says which (FEAT-121)', async () => {
		await planned('feat/tab-drag', free());
		activity.begin('pull', 'Pulling', ['repository', 'network']);

		expect(await merger.land()).toBe(false);

		expect(api.mergerLand).not.toHaveBeenCalled();
		expect(merger.landError).toBe('Pulling is still running');
		expect(merger.phase).not.toBe('done');
	});
});

describe('offering the push', () => {
	it('names the upstream of the branch that received it', async () => {
		await planned('feat/tab-drag', free());

		await land();

		expect(merger.pushTo).toEqual({ remote: 'origin', refspec: 'main', to: 'origin/main' });
		expect(merger.nowhereToPush).toBe(false);
	});

	it('pushes that branch to that remote by name, and waits for git to say how it went', async () => {
		await planned('feat/tab-drag', free());
		await land();
		let end: (ok: boolean) => void = () => {};
		net.whenDone.mockReturnValue(new Promise<boolean>((resolve) => (end = resolve)));

		const pushing = merger.pushLanded();
		await vi.waitFor(() => expect(net.whenDone).toHaveBeenCalled());
		expect(net.push).toHaveBeenCalledWith('origin', 'main');
		expect(merger.pushing).toBe(true);
		expect(merger.pushed).toBe(false);

		end(true);
		expect(await pushing).toBe(true);
		expect(merger.pushed).toBe(true);
		expect(merger.pushing).toBe(false);
	});

	it('does not depend on the checkout having worked', async () => {
		// The remote and the branch are named, so what is checked out is not
		// what gets pushed.
		await planned('feat/tab-drag', free());
		vi.mocked(api.checkout).mockRejectedValue('would be overwritten');
		await land();

		expect(await merger.pushLanded()).toBe(true);

		expect(net.push).toHaveBeenCalledWith('origin', 'main');
	});

	it("shows git's own words when the push is rejected, and is not marked pushed", async () => {
		await planned('feat/tab-drag', free());
		await land();
		net.whenDone.mockImplementation(async () => {
			net.error = '! [rejected] main -> main (fetch first)';
			return false;
		});

		expect(await merger.pushLanded()).toBe(false);

		expect(merger.pushed).toBe(false);
		expect(merger.pushError).toContain('fetch first');
		// Rejected, not forbidden: it can be pressed again after a pull.
		net.whenDone.mockResolvedValue(true);
		expect(await merger.pushLanded()).toBe(true);
		expect(merger.pushError).toBeNull();
	});

	it('waits its turn when a fetch, a push or a pull is running', async () => {
		await planned('feat/tab-drag', free());
		await land();
		activity.begin('pull', 'Pulling', ['repository', 'network']);

		expect(await merger.pushLanded()).toBe(false);

		expect(net.push).not.toHaveBeenCalled();
		expect(merger.pushError).toBe('Pulling is still running. Push again when it has finished.');
	});

	it('is pushed once: a second press while it is going does nothing', async () => {
		await planned('feat/tab-drag', free());
		await land();
		let end: (ok: boolean) => void = () => {};
		net.whenDone.mockReturnValue(new Promise<boolean>((resolve) => (end = resolve)));

		const first = merger.pushLanded();
		expect(await merger.pushLanded()).toBe(false);
		end(true);
		await first;
		expect(await merger.pushLanded()).toBe(false);

		expect(net.push).toHaveBeenCalledTimes(1);
	});

	it('says there is nowhere to push when the repository has no remote', async () => {
		vi.mocked(api.remotes).mockResolvedValue([]);
		await planned('feat/tab-drag', free());

		await land();

		expect(merger.pushTo).toBeNull();
		expect(merger.nowhereToPush).toBe(true);
		expect(await merger.pushLanded()).toBe(false);
		expect(net.push).not.toHaveBeenCalled();
	});

	it('offers nothing, and claims nothing, when the remotes could not be read', async () => {
		vi.mocked(api.remotes).mockRejectedValue('no repository is open');
		await planned('feat/tab-drag', free());

		await land();

		expect(merger.pushTo).toBeNull();
		expect(merger.nowhereToPush).toBe(false);
	});

	it('is forgotten when another merge is started', async () => {
		await planned('feat/tab-drag', free());
		await land();
		await merger.pushLanded();

		merger.again();

		expect(merger.checkout).toBeNull();
		expect(merger.pushTo).toBeNull();
		expect(merger.pushed).toBe(false);
		expect(merger.pushError).toBeNull();
	});

	it("leaves a pull request's own push as it was", async () => {
		// Its branch is already checked out, and it is pushed to the pull
		// request's branch, not to an upstream (BUG-076).
		vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h1', base: 'b1', mergeBase: 'm0' });
		vi.mocked(api.reviewCheckOut).mockResolvedValue({ branch: 'pr-61', upstream: null, renamed: true });
		vi.mocked(api.branches).mockResolvedValue([
			branchRow('pr-61', { current: true }),
			branchRow('origin/main', { kind: 'remote' }),
			branchRow('origin/feat/thing', { kind: 'remote' })
		]);
		vi.mocked(api.mergerForecast).mockResolvedValue(
			cleanForecast({ a: side({ name: 'pr-61' }), b: side({ name: 'origin/main', kind: 'remote', checkedOut: null }) })
		);
		vi.mocked(api.mergerLand).mockResolvedValue({ ...LANDED, target: 'pr-61' });
		merger.presentPullRequest(request({ number: 61, sourceBranch: 'feat/thing', targetBranch: 'main' }));
		await merger.resolvePullRequestLocally();

		expect(await merger.land()).toBe(true);
		await Promise.resolve();

		expect(api.checkout).not.toHaveBeenCalled();
		expect(merger.checkout).toBeNull();
		expect(merger.pushTo).toBeNull();
	});
});
