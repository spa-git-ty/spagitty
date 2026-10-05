// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, press, render, type Mounted } from '../../testing/mount';
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
	vi.mocked(api.reviewComments).mockResolvedValue([]);
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

it('opens a GitLab merge request from this repository', async () => {
	// BUG-044. GitLab's list names each row's project (`references.full`)
	// even when it is this repository's own list.
	vi.mocked(api.forgeRepo).mockResolvedValue({
		kind: 'gitLab',
		host: 'gitlab.example.com',
		owner: 'team',
		name: 'billing'
	});
	vi.mocked(api.forgeAccounts).mockResolvedValue([
		{ kind: 'gitLab', host: 'gitlab.example.com', user: 'mahmoud' }
	]);
	vi.mocked(api.pullRequests).mockResolvedValue([
		{ ...ASKED, repository: 'team/billing' }
	]);
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(1));

	click(button('Start review'));

	await vi.waitFor(() => expect(review.room?.pr.number).toBe(214));
	expect(api.localCloneOf).not.toHaveBeenCalled();
	expect(api.reviewState).toHaveBeenCalledWith({
		host: 'gitlab.example.com',
		owner: 'team',
		name: 'billing',
		number: 214
	});
});

it('says why a review did not open', async () => {
	const { notice } = await import('$lib/ui/notice.svelte');
	const elsewhere = request({ id: 'PR_77', number: 77, title: 'Elsewhere', repository: 'other/thing' });
	vi.mocked(api.involvedPullRequests).mockResolvedValue([elsewhere]);
	vi.mocked(api.localCloneOf).mockRejectedValue(new Error('the clone list could not be read'));
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));
	click(button('All my repos'));
	await vi.waitFor(() => expect(view.text()).toContain('Elsewhere'));

	click(button('Start review'));

	await vi.waitFor(() => expect(notice.current?.tone).toBe('error'));
	expect(notice.current?.title).toBe('The review could not be opened');
	expect(notice.current?.detail).toContain('the clone list could not be read');
	expect((button('Start review') as HTMLButtonElement).disabled).toBe(false);
	notice.dismiss();
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

it('asks GitLab for the checks and threads its list leaves out, once', async () => {
	vi.mocked(api.forgeRepo).mockResolvedValue({
		kind: 'gitLab',
		host: 'gitlab.example.com',
		owner: 'team/backend',
		name: 'payments'
	});
	vi.mocked(api.pullRequests).mockResolvedValue([{ ...ASKED, checks: null, openThreads: 0 }]);
	vi.mocked(api.reviewSummaries).mockResolvedValue([
		[null, { number: 214, checks: 'failing', openThreads: 2, resolvedThreads: 1, repliesToYou: 0 }]
	]);
	view = render(Page, {});

	await vi.waitFor(() => expect(view.text()).toContain('Checks failing'));
	expect(view.text()).toContain('2 open threads');
	expect(api.reviewSummaries).toHaveBeenCalledTimes(1);
	expect(api.reviewSummaries).toHaveBeenCalledWith([{ repository: null, number: 214 }]);
	expect(api.reviewState).toHaveBeenCalledWith({
		host: 'gitlab.example.com',
		owner: 'team/backend',
		name: 'payments',
		number: 214
	});
});

it('asks GitHub nothing more: its list already says', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));
	expect(api.reviewSummaries).not.toHaveBeenCalled();
});

it('checks the pull request out as a branch, fetching it first', async () => {
	// FEAT-095: it was Open in worktree.
	const { notice } = await import('$lib/ui/notice.svelte');
	vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'f00d', base: 'ba5e', mergeBase: 'b0b0' });
	vi.mocked(api.reviewCheckOut).mockResolvedValue({
		branch: 'feat/avatar-disk-cache',
		upstream: 'origin/feat/avatar-disk-cache',
		renamed: false
	});
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));
	expect(button('Open in worktree')).toBeUndefined();

	click(button('Check out branch'));
	await vi.waitFor(() =>
		expect(api.reviewCheckOut).toHaveBeenCalledWith(214, 'f00d', 'feat/avatar-disk-cache', 'main')
	);
	expect(api.reviewCheckout).toHaveBeenCalledWith(214, 'main', ASKED.headSha);
	await vi.waitFor(() => expect(notice.current?.title).toBe('On feat/avatar-disk-cache'));
	expect(notice.current?.detail).toBe('following origin/feat/avatar-disk-cache');
	expect(calls.refreshed).toBeGreaterThan(0);
	notice.dismiss();
});

it('says when the pull request had to take a name of its own', async () => {
	const { notice } = await import('$lib/ui/notice.svelte');
	vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'f00d', base: 'ba5e', mergeBase: 'b0b0' });
	vi.mocked(api.reviewCheckOut).mockResolvedValue({ branch: 'pr-214', upstream: null, renamed: true });
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));

	click(button('Check out branch'));
	await vi.waitFor(() => expect(notice.current?.title).toBe('On pr-214'));
	expect(notice.current?.detail).toBe('feat/avatar-disk-cache here is another branch');
	notice.dismiss();
});

it('says what git said when the branch cannot be checked out', async () => {
	const { notice } = await import('$lib/ui/notice.svelte');
	vi.mocked(api.reviewCheckout).mockResolvedValue({ head: 'f00d', base: 'ba5e', mergeBase: 'b0b0' });
	vi.mocked(api.reviewCheckOut).mockRejectedValue(
		new Error('Your local changes to the following files would be overwritten by checkout')
	);
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(2));

	click(button('Check out branch'));
	await vi.waitFor(() => expect(notice.current?.tone).toBe('error'));
	expect(notice.current?.title).toBe('The branch could not be checked out');
	expect(notice.current?.detail).toContain('would be overwritten');
	notice.dismiss();
});

it('draws the description as its host does, and widens from its edge', async () => {
	// FEAT-094: dependabot's table, as a table.
	const { panels } = await import('$lib/panels.svelte');
	vi.mocked(api.pullRequests).mockResolvedValue([
		{
			...ASKED,
			body: 'Bumps the group:\n\n| Package | From | To |\n| --- | --- | --- |\n| ktor | `3.5.2` | `3.6.0` |'
		}
	]);
	view = render(Page, {});
	await vi.waitFor(() => expect(cards()).toHaveLength(1));
	const preview = view.get('aside[aria-label="Pull request preview"]');
	expect([...preview.querySelectorAll('th')].map((th) => th.textContent)).toEqual(['Package', 'From', 'To']);
	expect(preview.textContent).not.toContain('| --- |');

	const edge = view.get('[role="separator"][aria-label="Resize the preview"]');
	const before = panels.size('reviewPreview');
	press(edge, 'ArrowLeft');
	expect(panels.size('reviewPreview')).toBe(before + 8);
	panels.reset();
});
