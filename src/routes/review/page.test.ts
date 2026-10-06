// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { openRepository, request } from '../../testing/git-fixtures';
import { calls, control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from '$lib/api';
import { goto } from '$app/navigation';
import { requests } from '$lib/requests/store.svelte';
import { review } from '$lib/review/store.svelte';
import Page from './+page.svelte';

let view: Mounted;
const button = (name: string) =>
	view.all('button').find((b) => b.textContent?.replace(/\s+/g, ' ').trim() === name)!;
const cards = () => view.all('button.row');

const ASKED = request({
	id: 'PR_214',
	number: 214,
	title: 'Cache avatars on disk instead of in memory',
	authorName: 'yasser-dev',
	sourceBranch: 'feat/avatar-disk-cache',
	reviewRequested: true,
	openThreads: 2,
	resolvedThreads: 1,
	changedFiles: 6,
	added: 16,
	removed: 8
});
const ANSWERED = request({
	id: 'PR_201',
	number: 201,
	title: 'Rebase screen: keep the todo list when the window loses focus',
	authorName: 'omar.k',
	repliesToYou: 1
});
const MINE = request({ id: 'PR_9', number: 9, title: 'My own change', authorName: 'mahmoud' });

beforeEach(() => {
	vi.clearAllMocks();
	requests.clear();
	review.clear();
	control.reset();
	openRepository();
	localStorage.clear();
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
	vi.mocked(api.pullRequests).mockResolvedValue([MINE, ANSWERED, ASKED]);
	vi.mocked(api.reviewState).mockResolvedValue(null);
});

afterEach(() => {
	view?.destroy();
	requests.clear();
	review.clear();
	control.reset();
});

it('groups what needs you above what came back, and leaves your own out', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));

	const text = view.text();
	expect(text).toContain('Needs you · you are a requested reviewer');
	expect(text).toContain('Back with you · the author answered you');
	expect(text.indexOf('#214')).toBeLessThan(text.indexOf('#201'));
	expect(text).not.toContain('My own change');
	expect(text).toContain('2 open threads');
	expect(text).toContain('1 reply to you');
	expect(text).toContain('spagitty · GitHub · signed in as mahmoud');
});

it('previews the first pull request and then the one chosen', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));

	const preview = () => view.get('aside[aria-label="Pull request preview"]').textContent ?? '';
	expect(preview()).toContain('#214 · feat/avatar-disk-cache → main');
	expect(preview()).toContain('2 open threads, 1 resolved');
	expect(preview()).toContain('Start review');

	click(cards()[1]);
	expect(preview()).toContain('Rebase screen: keep the todo list');
});

it('shows how far a saved review got, and goes on from there', async () => {
	vi.mocked(api.reviewState).mockImplementation(async (key) =>
		key.number === 214
			? { headSha: ASKED.headSha, viewed: { a: '1', b: '2', c: '3' }, files: 6 }
			: null
	);
	view = render(Page, {});
	await vi.waitFor(() => expect(view.text()).toContain('3 of 6 viewed'));

	expect(api.reviewState).toHaveBeenCalledWith({
		host: 'github.com',
		owner: 'spa-git-ty',
		name: 'spagitty',
		number: 214
	});
	click(button('Continue review · 3 of 6 viewed'));
	await vi.waitFor(() => expect(review.room?.pr.number).toBe(214));
	expect(view.text()).toContain('yasser-dev wants to merge');

	click(button('Review'));
	expect(review.room).toBeNull();
	expect(view.text()).toContain('Needs you');
});

it('reads every repository on asking, and says when a row has no clone here', async () => {
	const elsewhere = request({
		id: 'PR_77',
		number: 77,
		title: 'Elsewhere',
		reviewRequested: true,
		repository: 'other/thing'
	});
	vi.mocked(api.involvedPullRequests).mockResolvedValue([elsewhere]);
	vi.mocked(api.localCloneOf).mockResolvedValue(null);
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));

	click(button('All my repos'));
	await vi.waitFor(() => expect(view.text()).toContain('Elsewhere'));
	expect(view.text()).toContain('other/thing');

	click(button('Start review'));
	await vi.waitFor(() => expect(view.text()).toContain('No clone of other/thing here'));
	expect(api.localCloneOf).toHaveBeenCalledWith('github.com', 'other/thing');
	expect(review.room).toBeNull();
});

it('opens the clone a row from another repository belongs to', async () => {
	const elsewhere = request({ id: 'PR_77', number: 77, title: 'Elsewhere', repository: 'other/thing' });
	vi.mocked(api.involvedPullRequests).mockResolvedValue([elsewhere]);
	vi.mocked(api.localCloneOf).mockResolvedValue('/repos/thing');
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));
	click(button('All my repos'));
	await vi.waitFor(() => expect(view.text()).toContain('Elsewhere'));

	vi.mocked(api.forgeRepo).mockResolvedValue({
		kind: 'gitHub',
		host: 'github.com',
		owner: 'other',
		name: 'thing'
	});
	vi.mocked(api.pullRequests).mockResolvedValue([{ ...elsewhere, repository: null }]);
	click(button('Start review'));

	await vi.waitFor(() => expect(review.room?.pr.number).toBe(77));
	expect(calls.opened).toContain('/repos/thing');
	expect(review.scope).toBe('repo');
});

it('says what the host said, and sends account trouble to Settings', async () => {
	vi.mocked(api.pullRequests).mockRejectedValue(new Error('the token was refused'));
	view = render(Page, {});
	await vi.waitFor(() => expect(view.text()).toContain('the token was refused'));

	click(button('Settings → Accounts'));
	expect(goto).toHaveBeenCalledWith('/settings#accounts');
});

it('says so when nothing is waiting', async () => {
	vi.mocked(api.pullRequests).mockResolvedValue([MINE]);
	view = render(Page, {});
	await vi.waitFor(() => expect(view.text()).toContain('Nothing to review.'));
});
