// SPDX-License-Identifier: GPL-3.0-or-later

/** The band at the top of Merger when it was opened for something (FEAT-117). */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import { request } from '../../testing/git-fixtures';
import { branchRow, forecast } from '../../testing/merger-fixtures';

vi.mock('$app/navigation', () => ({ goto: vi.fn(() => Promise.resolve()) }));
vi.mock('$lib/api');
vi.mock('$lib/farm/api');

import * as api from '$lib/api';
import { requests } from '$lib/requests/store.svelte';
import MergerOrigin from './MergerOrigin.svelte';
import { merger } from './store.svelte';

const text = (el: Element) => el.textContent?.replace(/\s+/g, ' ').trim() ?? '';

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	requests.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
});

afterEach(() => merger.reset());

describe('a pull request', () => {
	it('offers each method the host has, merges with the one chosen, and says it is merged', async () => {
		vi.mocked(api.mergePullRequest).mockResolvedValue(undefined);
		merger.presentPullRequest(request({ number: 61, title: 'Fix the thing', mergeable: true }));
		const view = render(MergerOrigin, {});

		expect(view.text()).toContain('#61');
		expect(view.text()).toContain('Fix the thing');
		for (const method of ['Merge commit', 'Squash', 'Rebase']) expect(view.text()).toContain(method);
		expect(view.text()).not.toContain('Resolve the conflicts here');

		click(view.all('button').find((b) => text(b) === 'Squash')!);
		flushSync();
		click(view.all('button').find((b) => text(b) === 'Merge pull request')!);
		await vi.waitFor(() => expect(api.mergePullRequest).toHaveBeenCalledWith(61, 'squash', undefined, undefined));
		await vi.waitFor(() => expect(view.text()).toContain('#61 is merged into main'));
		view.destroy();
	});

	it('shows the host’s refusal and stays open', async () => {
		vi.mocked(api.mergePullRequest).mockRejectedValue('github.com: Pull Request is not mergeable');
		merger.presentPullRequest(request({ number: 61 }));
		const view = render(MergerOrigin, {});

		click(view.all('button').find((b) => text(b) === 'Merge pull request')!);
		await vi.waitFor(() => expect(view.text()).toContain('not mergeable'));
		expect(view.text()).not.toContain('is merged');
		view.destroy();
	});

	it('offers to resolve its conflicts here when the host says it conflicts', () => {
		merger.presentPullRequest(request({ number: 61, mergeable: false }));
		const view = render(MergerOrigin, {});

		expect(view.text()).toContain('Resolve the conflicts here');
		expect(view.text()).toContain('checks the pull request');
		view.destroy();
	});

	it('says what is being resolved, and on which branch, once it is', async () => {
		vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'h1', base: 'b1', mergeBase: 'm0' });
		vi.mocked(api.reviewCheckOut).mockResolvedValue({ branch: 'feat/thing', upstream: null, renamed: false });
		vi.mocked(api.branches).mockResolvedValue([branchRow('feat/thing', { current: true }), branchRow('main')]);
		vi.mocked(api.tags).mockResolvedValue([]);
		vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
		merger.presentPullRequest(request({ number: 61, mergeable: false, sourceBranch: 'feat/thing', targetBranch: 'main' }));
		const view = render(MergerOrigin, {});

		await merger.resolvePullRequestLocally();
		flushSync();

		expect(view.text()).toContain('Resolving here, on feat/thing');
		// The methods and the host's merge come after it is pushed, not now.
		expect(view.text()).not.toContain('Merge pull request');
		view.destroy();
	});
});

describe('a Farm task', () => {
	it('says which task and which branches', () => {
		merger.presentFarmTask({ id: 'T3', branch: 'farm/T3-work', target: 'main', title: 'The work' });
		const view = render(MergerOrigin, {});

		expect(view.text()).toContain('T3');
		expect(view.text()).toContain('The work');
		expect(view.text()).toContain('farm/T3-work → main');
		view.destroy();
	});
});

describe('nothing in particular', () => {
	it('draws nothing', () => {
		const view = render(MergerOrigin, {});
		expect(view.target.querySelector('.origin')).toBeNull();
		view.destroy();
	});
});
