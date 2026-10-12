// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { BranchRow } from '$lib/types';

vi.mock('$lib/api', () => ({
	branches: vi.fn(),
	checkout: vi.fn(),
	createBranch: vi.fn(),
	deleteBranch: vi.fn(),
	renameBranch: vi.fn()
}));

vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));

import * as api from '$lib/api';
import { calls as repoCalls, control as repoControl } from '../../testing/repo-store.svelte';
import { activity } from '$lib/activity/store.svelte';
import { branches, STALE_DAYS } from './store.svelte';

const list = vi.mocked(api.branches);
const checkout = vi.mocked(api.checkout);
const createBranch = vi.mocked(api.createBranch);
const deleteBranch = vi.mocked(api.deleteBranch);
const renameBranch = vi.mocked(api.renameBranch);

const NOW = 1_800_000_000;

function row(name: string, overrides: Partial<BranchRow> = {}): BranchRow {
	return {
		name,
		fullName: `refs/heads/${name}`,
		kind: 'branch',
		current: false,
		id: 'a'.repeat(40),
		short: 'aaaaaaa',
		summary: `tip of ${name}`,
		authorName: 'Ada Lovelace',
		time: NOW - 60,
		upstream: null,
		ahead: null,
		behind: null,
		merged: false,
		...overrides
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
	branches.clear();
	repoControl.reset();
	activity.reset();
	vi.useFakeTimers({ now: NOW * 1000, toFake: ['Date'] });
	list.mockResolvedValue([
		row('main', { current: true, upstream: 'origin/main', ahead: 2, behind: 1 }),
		row('chore/tooling'),
		row('merged/old', { merged: true, time: NOW - (STALE_DAYS + 10) * 86400 }),
		row('origin/main', { kind: 'remote', fullName: 'refs/remotes/origin/main' })
	]);
	checkout.mockResolvedValue(undefined);
	createBranch.mockResolvedValue(undefined);
});

describe('load', () => {
	it('takes the rows as the core ordered them', async () => {
		await branches.load();

		expect(branches.rows.map((r) => r.name)).toEqual([
			'main',
			'chore/tooling',
			'merged/old',
			'origin/main'
		]);
		expect(branches.loaded).toBe(true);
		expect(branches.error).toBeNull();
	});

	it('knows which row is the current branch', async () => {
		await branches.load();
		expect(branches.current?.name).toBe('main');
	});

	it('records a failure and shows nothing', async () => {
		list.mockRejectedValueOnce('could not read refs');
		await branches.load();

		expect(branches.error).toBe('could not read refs');
		expect(branches.rows).toEqual([]);
		expect(branches.loading).toBe(false);
	});

	it('drops a slow read that a newer one superseded', async () => {
		const slow = deferred<BranchRow[]>();
		list.mockReturnValueOnce(slow.promise);

		const first = branches.load();
		await branches.load();
		slow.resolve([row('stale-result')]);
		await first;
		await settle();

		expect(branches.rows.map((r) => r.name)).not.toContain('stale-result');
	});
});

describe('filtering', () => {
	it('shows everything with no filter', async () => {
		await branches.load();
		expect(branches.filtered).toHaveLength(4);
		expect(branches.hidden).toBe(0);
	});

	it('matches the name and the upstream, case-insensitively', async () => {
		await branches.load();

		branches.setQuery('CHORE');
		expect(branches.filtered.map((r) => r.name)).toEqual(['chore/tooling']);

		// "origin/ma" is a reasonable thing to type while looking for `main`.
		branches.setQuery('origin/ma');
		expect(branches.filtered.map((r) => r.name)).toEqual(['main', 'origin/main']);
	});

	it('separates local from remote-tracking', async () => {
		await branches.load();

		branches.toggle('mine');
		expect(branches.filtered.every((r) => r.kind === 'branch')).toBe(true);

		branches.toggle('mine');
		branches.toggle('origin');
		expect(branches.filtered.map((r) => r.name)).toEqual(['origin/main']);
	});

	it('finds the ones with an upstream, and the merged ones', async () => {
		await branches.load();

		branches.toggle('upstream');
		expect(branches.filtered.map((r) => r.name)).toEqual(['main']);

		branches.clearFilters();
		branches.toggle('merged');
		expect(branches.filtered.map((r) => r.name)).toEqual(['merged/old']);
	});

	it('calls a branch stale only after the whole window has passed', async () => {
		list.mockResolvedValueOnce([
			row('just-inside', { time: NOW - (STALE_DAYS - 1) * 86400 }),
			row('just-outside', { time: NOW - (STALE_DAYS + 1) * 86400 })
		]);
		await branches.load();

		branches.toggle('stale');

		expect(branches.filtered.map((r) => r.name)).toEqual(['just-outside']);
	});

	it('composes chips as AND', async () => {
		await branches.load();

		branches.toggle('merged');
		branches.toggle('stale');
		expect(branches.filtered.map((r) => r.name)).toEqual(['merged/old']);

		branches.toggle('upstream');
		expect(branches.filtered).toEqual([]);
	});

	it('counts what the filters are hiding', async () => {
		await branches.load();
		branches.toggle('origin');

		expect(branches.hidden).toBe(3);
	});

	it('clears the text and the chips together', async () => {
		await branches.load();
		branches.setQuery('chore');
		branches.toggle('merged');

		branches.clearFilters();

		expect(branches.query).toBe('');
		expect(branches.active).toEqual([]);
		expect(branches.filtered).toHaveLength(4);
	});
});

describe('checkout', () => {
	it('checks out and re-reads everything it could have changed', async () => {
		await branches.load();
		list.mockClear();

		expect(await branches.checkout('chore/tooling')).toBe(true);

		expect(checkout).toHaveBeenCalledWith('chore/tooling');
		expect(list).toHaveBeenCalled();
		expect(repoCalls.refreshed).toBeGreaterThan(0);
	});

	it('surfaces the refusal rather than pretending it worked', async () => {
		await branches.load();
		checkout.mockRejectedValueOnce(
			'git switch failed: Your local changes would be overwritten'
		);

		expect(await branches.checkout('chore/tooling')).toBe(false);
		expect(branches.writeError).toContain('would be overwritten');
	});

	it('refuses a second write while one is in flight', async () => {
		await branches.load();
		const slow = deferred<void>();
		checkout.mockReturnValueOnce(slow.promise);

		const first = branches.checkout('a');
		expect(await branches.checkout('b')).toBe(false);
		expect(checkout).toHaveBeenCalledTimes(1);

		slow.resolve();
		await first;
	});
});

describe('said while it runs (FEAT-121)', () => {
	it('names a switch for as long as git takes', async () => {
		const git = deferred<void>();
		checkout.mockReturnValueOnce(git.promise);

		const running = branches.checkout('chore/tooling');
		expect(activity.label).toBe('Switching to chore/tooling…');
		expect(activity.is('checkout')).toBe(true);

		git.resolve();
		await running;
		expect(activity.running).toBe(false);
	});

	it('lets go when git refuses the switch', async () => {
		checkout.mockRejectedValueOnce('your local changes would be overwritten');

		await branches.checkout('chore/tooling');

		expect(activity.running).toBe(false);
		expect(branches.writeError).toBe('your local changes would be overwritten');
	});

	it('does not switch over a write still running, and says which', async () => {
		activity.begin('pull', 'Pulling', ['repository', 'network']);

		expect(await branches.checkout('chore/tooling')).toBe(false);

		expect(checkout).not.toHaveBeenCalled();
		expect(branches.writeError).toBe('Pulling is still running');
		expect(branches.busy).toBe(false);
	});

	it('switches while only the network is busy', async () => {
		activity.begin('fetch', 'Fetching', ['network']);

		expect(await branches.checkout('chore/tooling')).toBe(true);
	});

	it('names the other writes too', async () => {
		const said: string[] = [];
		renameBranch.mockImplementationOnce(async () => void said.push(activity.label));
		deleteBranch.mockImplementation(async () => void said.push(activity.label));
		createBranch.mockImplementationOnce(async () => void said.push(activity.label));

		await branches.rename('chore/tooling', 'chore/tools');
		await branches.delete('merged/old', false);
		await branches.deleteMany(['a', 'b'], false);
		branches.setNewName('topic');
		await branches.create();

		expect(said).toEqual([
			'Renaming chore/tooling…',
			'Deleting merged/old…',
			'Deleting 2 branches…',
			'Deleting 2 branches…',
			'Creating topic…'
		]);
	});
});

describe('create', () => {
	it('creates from the form and clears it', async () => {
		await branches.load();
		branches.setNewName('  chore/new  ');
		branches.setNewStart(' main ');

		expect(await branches.create()).toBe(true);

		expect(createBranch).toHaveBeenCalledWith('chore/new', 'main', true);
		expect(branches.newName).toBe('');
		expect(branches.newStart).toBe('');
	});

	it('sends an empty start point, which the core reads as HEAD', async () => {
		await branches.load();
		branches.setNewName('chore/new');

		await branches.create();

		expect(createBranch).toHaveBeenCalledWith('chore/new', '', true);
	});

	it('can create without checking out', async () => {
		await branches.load();
		branches.setNewName('chore/new');
		branches.setNewCheckout(false);

		await branches.create();

		expect(createBranch).toHaveBeenCalledWith('chore/new', '', false);
	});

	it('does nothing without a name', async () => {
		await branches.load();
		branches.setNewName('   ');

		expect(await branches.create()).toBe(false);
		expect(createBranch).not.toHaveBeenCalled();
	});

	it('keeps the form when git refuses the name', async () => {
		await branches.load();
		branches.setNewName('has spaces');
		createBranch.mockRejectedValueOnce("git branch failed: 'has spaces' is not a valid name");

		expect(await branches.create()).toBe(false);

		expect(branches.newName).toBe('has spaces');
		expect(branches.writeError).toContain('not a valid name');
	});
});

describe('clear', () => {
	it('forgets the rows, the filters and the form', async () => {
		await branches.load();
		branches.setQuery('chore');
		branches.toggle('merged');
		branches.setNewName('half-typed');

		branches.clear();

		expect(branches.rows).toEqual([]);
		expect(branches.loaded).toBe(false);
		expect(branches.query).toBe('');
		expect(branches.active).toEqual([]);
		expect(branches.newName).toBe('');
	});
});

describe('the destructive writes', () => {
	beforeEach(() => {
		deleteBranch.mockResolvedValue(undefined);
		renameBranch.mockResolvedValue(undefined);
		list.mockResolvedValue([]);
	});

	it('deletes a branch, forcing only when told to', async () => {
		expect(await branches.delete('feature/live', true)).toBe(true);
		expect(deleteBranch).toHaveBeenCalledWith('feature/live', true);

		await branches.delete('merged/old', false);
		expect(deleteBranch).toHaveBeenLastCalledWith('merged/old', false);
	});

	it('re-reads the list and the repository afterwards', async () => {
		list.mockClear();
		await branches.delete('feature/live', false);

		// A deleted branch changes what every other row's merged flag means.
		expect(list).toHaveBeenCalled();
		expect(repoCalls.refreshed).toBeGreaterThan(0);
	});

	it('surfaces a refusal rather than pretending it worked', async () => {
		deleteBranch.mockRejectedValueOnce(new Error('not fully merged'));

		expect(await branches.delete('feature/live', false)).toBe(false);
		expect(branches.writeError).toContain('not fully merged');
	});

	it('renames, trimming what was typed', async () => {
		expect(await branches.rename('old', '  new  ')).toBe(true);
		expect(renameBranch).toHaveBeenCalledWith('old', 'new');
	});

	it('refuses a rename to nothing or to the same name', async () => {
		expect(await branches.rename('old', '   ')).toBe(false);
		expect(await branches.rename('old', 'old')).toBe(false);
		expect(renameBranch).not.toHaveBeenCalled();
	});

	it('deletes several branches one at a time', async () => {
		// `git branch -d` takes a lock; running them at once is how a bulk
		// cleanup half-happens.
		await branches.deleteMany(['a', 'b', 'c'], false);

		expect(deleteBranch.mock.calls.map((call) => call[0])).toEqual(['a', 'b', 'c']);
	});

	it('stops a bulk delete at the first refusal', async () => {
		deleteBranch.mockResolvedValueOnce(undefined);
		deleteBranch.mockRejectedValueOnce(new Error('not fully merged'));

		expect(await branches.deleteMany(['a', 'b', 'c'], false)).toBe(false);
		// `c` was never attempted: carrying on would leave a list of things
		// that did not happen for reasons nobody saw.
		expect(deleteBranch).toHaveBeenCalledTimes(2);
		expect(branches.writeError).toContain('not fully merged');
	});

	it('does nothing for an empty bulk delete', async () => {
		expect(await branches.deleteMany([], false)).toBe(false);
		expect(deleteBranch).not.toHaveBeenCalled();
	});
});
