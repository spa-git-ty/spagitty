// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ForgeRepo, PullRequest } from '$lib/types';

vi.mock('$lib/api', () => ({
	inTauri: vi.fn(() => true),
	forgeRepo: vi.fn(),
	forgeAccounts: vi.fn(() => Promise.resolve([])),
	pullRequests: vi.fn(),
	pullRequestFiles: vi.fn(),
	pullRequestCommits: vi.fn(),
	pullRequestComments: vi.fn(),
	commitFiles: vi.fn(),
	submitReview: vi.fn(),
	replyComment: vi.fn()
}));

import * as api from '$lib/api';
import { requests } from './store.svelte';

const forgeRepo = vi.mocked(api.forgeRepo);
const pullRequests = vi.mocked(api.pullRequests);

const REPO: ForgeRepo = {
	kind: 'gitHub',
	host: 'github.com',
	owner: 'spagitty',
	name: 'spagitty'
};

function request(overrides: Partial<PullRequest> = {}): PullRequest {
	return {
		id: 'PR_1',
		number: 412,
		title: 'Give the graph a footer',
		body: 'PR body markdown',
		authorName: 'grace',
		updated: 1_787_650_200,
		sourceBranch: 'feature/footer',
		targetBranch: 'main',
		draft: false,
		review: 'awaitingReview',
		checks: 'passing',
		needsYou: false,
		needsYouBecause: null,
		changedFiles: 7,
		added: 120,
		removed: 34,
		mergeable: true,

		headSha: 'a1b2c3d4e5f6',

		reviewRequested: false,

		openThreads: 0,

		resolvedThreads: 0,

		repliesToYou: 0,

		repository: null,
		...overrides
	};
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
	vi.clearAllMocks();
	requests.clear();
	// Re-established rather than left to the factory: `clearAllMocks` does not
	// drain a queued `mockReturnValueOnce`, and one test sets a false here.
	vi.mocked(api.inTauri).mockReturnValue(true);
	forgeRepo.mockResolvedValue(REPO);
	pullRequests.mockResolvedValue([request()]);
});

describe('load', () => {
	it('reads the repository first, and only then the pull requests', async () => {
		await requests.load();

		expect(forgeRepo).toHaveBeenCalled();
		expect(pullRequests).toHaveBeenCalled();
		expect(requests.all).toHaveLength(1);
		expect(requests.connected).toBe(true);
		expect(requests.repo).toEqual(REPO);
	});

	it('does not ask a host about a repository that is not on one', async () => {
		// The commonest case for anybody with a repository on a NAS or a bare
		// path. Nothing to fetch is not a failed fetch, and making the request
		// anyway would be a request nobody could have answered.
		forgeRepo.mockResolvedValueOnce(null);

		await requests.load();

		expect(pullRequests).not.toHaveBeenCalled();
		expect(requests.repo).toBeNull();
		expect(requests.error).toBeNull();
		expect(requests.all).toEqual([]);
	});

	it('opens the first request so the detail panel is not empty beside a list', async () => {
		pullRequests.mockResolvedValueOnce([request({ id: 'a' }), request({ id: 'b' })]);

		await requests.load();

		expect(requests.openId).toBe('a');
	});

	it('keeps the open request across a re-read while it is still there', async () => {
		pullRequests.mockResolvedValue([request({ id: 'a' }), request({ id: 'b' })]);
		await requests.load();
		requests.select('b');

		await requests.load();

		expect(requests.openId).toBe('b');
	});

	it('moves on when the request that was open has been merged away', async () => {
		pullRequests.mockResolvedValueOnce([request({ id: 'a' }), request({ id: 'b' })]);
		await requests.load();
		requests.select('b');

		pullRequests.mockResolvedValueOnce([request({ id: 'a' })]);
		await requests.load();

		expect(requests.openId).toBe('a');
	});

	it('surfaces the host’s own sentence rather than a generic failure', async () => {
		// Offline, rate limited, refused and no-account are four different
		// decisions for the reader, and the backend already told them apart.
		// Flattening them to "could not load" here would throw that away.
		pullRequests.mockRejectedValueOnce(
			'github.com is rate limiting — it should accept requests again in 60 seconds'
		);

		await requests.load();

		expect(requests.error).toContain('rate limiting');
		expect(requests.error).toContain('60 seconds');
		expect(requests.connected).toBe(false);
		expect(requests.all).toEqual([]);
	});

	it('says a failure is a failure rather than showing a stale list', async () => {
		await requests.load();
		expect(requests.all).toHaveLength(1);

		pullRequests.mockRejectedValueOnce('could not reach github.com');
		await requests.load();

		expect(requests.all).toEqual([]);
		expect(requests.openId).toBeNull();
	});

	it('reports while it is reading and stops when it is done', async () => {
		let release: (value: PullRequest[]) => void = () => {};
		pullRequests.mockReturnValueOnce(new Promise((resolve) => (release = resolve)));

		const reading = requests.load();
		await settle();
		expect(requests.loading).toBe(true);

		release([request()]);
		await reading;
		expect(requests.loading).toBe(false);
	});

	it('ignores a slow read that lost the race to a newer one', async () => {
		// Switching repository quickly. The first answer must not land on the
		// second repository's screen.
		let release: (value: PullRequest[]) => void = () => {};
		pullRequests.mockReturnValueOnce(new Promise((resolve) => (release = resolve)));

		const stale = requests.load();
		// Let the repository read resolve, so the request is actually in flight.
		// Without this the load returns at the guard *before* asking for pull
		// requests, and the race under test never happens.
		await settle();
		requests.clear();

		release([request({ id: 'stale' })]);
		await stale;

		expect(requests.all).toEqual([]);
		expect(requests.loading).toBe(false);
	});

	it('does nothing at all outside the application', async () => {
		// The dev server in a plain browser has no backend to ask.
		vi.mocked(api.inTauri).mockReturnValueOnce(false);

		await requests.load();

		expect(forgeRepo).not.toHaveBeenCalled();
		expect(pullRequests).not.toHaveBeenCalled();
	});
});

describe('what the screen leads with', () => {
	it('separates what is waiting on you from what is waiting on others', async () => {
		pullRequests.mockResolvedValue([
			request({ id: 'mine', needsYou: true, needsYouBecause: 'your review was requested' }),
			request({ id: 'theirs', needsYou: false })
		]);

		await requests.load();

		expect(requests.needingYou.map((r) => r.id)).toEqual(['mine']);
		expect(requests.waitingOnOthers.map((r) => r.id)).toEqual(['theirs']);
	});
});
