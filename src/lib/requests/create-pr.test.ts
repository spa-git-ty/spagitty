// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Pull request creation tests (FEAT-070).
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { PullRequest } from '$lib/types';

vi.mock('$lib/api', () => ({
	createPullRequest: vi.fn(),
	forgeRepo: vi.fn(),
	forgeAccounts: vi.fn(),
	pullRequests: vi.fn(),
	inTauri: () => true
}));

import * as api from '$lib/api';
import { requests } from './store.svelte';

const createPullRequest = vi.mocked(api.createPullRequest);
const forgeRepo = vi.mocked(api.forgeRepo);
const pullRequests = vi.mocked(api.pullRequests);

beforeEach(() => {
	vi.clearAllMocks();
	requests.clear();
});

describe('create pull request flow', () => {
	it('toggles create modal visibility', () => {
		expect(requests.isCreateModalOpen).toBe(false);
		requests.openCreateModal();
		expect(requests.isCreateModalOpen).toBe(true);
		requests.closeCreateModal();
		expect(requests.isCreateModalOpen).toBe(false);
	});

	it('creates a pull request and refreshes list', async () => {
		const samplePR: PullRequest = {
			id: 'PR_123',
			number: 42,
			title: 'Add new feature',
			body: 'Description text',
			authorName: 'developer',
			updated: 1700000000,
			sourceBranch: 'feature/new-feature',
			targetBranch: 'main',
			draft: false,
			review: 'awaitingReview',
			checks: null,
			needsYou: false,
			needsYouBecause: null,
			changedFiles: 3,
			added: 50,
			removed: 10,
			mergeable: true,

			headSha: 'a1b2c3d4e5f6',

			reviewRequested: false,

			openThreads: 0,

			resolvedThreads: 0,

			repliesToYou: 0,

			repository: null
		};

		forgeRepo.mockResolvedValueOnce({
			kind: 'gitHub',
			host: 'github.com',
			owner: 'owner',
			name: 'repo'
		});
		createPullRequest.mockResolvedValueOnce(samplePR);
		pullRequests.mockResolvedValueOnce([samplePR]);

		const result = await requests.create(
			'Add new feature',
			'Description text',
			'feature/new-feature',
			'main',
			false
		);

		expect(result).toEqual(samplePR);
		expect(createPullRequest).toHaveBeenCalledWith(
			'Add new feature',
			'Description text',
			'feature/new-feature',
			'main',
			false
		);
		expect(requests.isCreateModalOpen).toBe(false);
	});
});
