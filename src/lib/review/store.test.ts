// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { openRepository, request } from '../../testing/git-fixtures';
import { calls, control } from '../../testing/repo-store.svelte';

vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
import * as api from '$lib/api';
import { requests } from '$lib/requests/store.svelte';
import { review } from './store.svelte';

const elsewhere = request({
	id: 'PR_77',
	number: 77,
	title: 'Elsewhere',
	repository: 'other/thing'
});

beforeEach(async () => {
	vi.clearAllMocks();
	requests.clear();
	review.clear();
	control.reset();
	openRepository();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.forgeRepo).mockResolvedValue({
		kind: 'gitHub',
		host: 'github.com',
		owner: 'spa-git-ty',
		name: 'spagitty'
	});
	vi.mocked(api.forgeAccounts).mockResolvedValue([
		{ kind: 'gitHub', host: 'github.com', user: 'mahmoud' }
	]);
	vi.mocked(api.pullRequests).mockResolvedValue([]);
	vi.mocked(api.involvedPullRequests).mockResolvedValue([elsewhere]);
	vi.mocked(api.reviewState).mockResolvedValue(null);
	await requests.load();
});

afterEach(() => {
	requests.clear();
	review.clear();
	control.reset();
});

// BUG-056 removes the scope control from the screen. Keep the store's existing
// cross-repository behavior covered without relying on that removed control.
it('reads involved pull requests and says when another repository has no local clone', async () => {
	vi.mocked(api.localCloneOf).mockResolvedValue(null);
	review.setScope('all');
	await vi.waitFor(() => expect(review.loading).toBe(false));
	expect(review.list).toEqual([elsewhere]);

	expect(await review.open(elsewhere)).toBe(false);
	expect(api.localCloneOf).toHaveBeenCalledWith('github.com', 'other/thing');
	expect(review.notHere).toBe('other/thing');
	expect(review.room).toBeNull();
});

it('propagates a clone lookup failure without opening a review room', async () => {
	vi.mocked(api.localCloneOf).mockRejectedValue(new Error('the clone list could not be read'));
	await expect(review.open(elsewhere)).rejects.toThrow('the clone list could not be read');
	expect(review.room).toBeNull();
});

it('opens the known clone and its pull request in repository scope', async () => {
	vi.mocked(api.localCloneOf).mockResolvedValue('/repos/thing');
	vi.mocked(api.forgeRepo).mockResolvedValue({
		kind: 'gitHub',
		host: 'github.com',
		owner: 'other',
		name: 'thing'
	});
	vi.mocked(api.pullRequests).mockResolvedValue([{ ...elsewhere, repository: null }]);

	expect(await review.open(elsewhere)).toBe(true);
	expect(calls.opened).toContain('/repos/thing');
	expect(review.scope).toBe('repo');
	expect(review.room?.pr.number).toBe(77);
	expect(review.room?.key).toEqual({
		host: 'github.com',
		owner: 'other',
		name: 'thing',
		number: 77
	});
});

it('does not ask for involved pull requests without an account for the current host', async () => {
	vi.mocked(api.forgeAccounts).mockResolvedValue([
		{ kind: 'gitLab', host: 'gitlab.example.com', user: 'mahmoud' }
	]);
	review.setScope('all');
	await vi.waitFor(() => expect(review.loading).toBe(false));

	expect(review.connected).toBe(false);
	expect(review.error).toBeNull();
	expect(review.list).toEqual([]);
	expect(api.involvedPullRequests).not.toHaveBeenCalled();
});
