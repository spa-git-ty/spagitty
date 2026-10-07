// SPDX-License-Identifier: GPL-3.0-or-later

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render, type Mounted } from '../../testing/mount';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/rebase/actions');
vi.mock('$lib/branches/store.svelte', () => ({
	branches: {
		rows: [
			{ name: 'topic', kind: 'branch', current: true, short: 'ccccccc', summary: 'mine', time: 100, upstream: null },
			{ name: 'main', kind: 'branch', current: false, short: 'bbbbbbb', summary: 'Their tip', time: 90, upstream: null },
			{ name: 'origin/main', kind: 'remote', current: false, short: 'bbbbbbb', summary: 'Their tip', time: 90, upstream: null }
		],
		load: vi.fn(() => Promise.resolve())
	}
}));

import * as api from '$lib/api';
import { goto } from '$app/navigation';
import * as actions from '$lib/rebase/actions';
import { rebase } from '$lib/rebase/store.svelte';
import { control as repoControl } from '../../testing/repo-store.svelte';
import Page from './+page.svelte';

let view: Mounted;
const button = (name: string) =>
	view.all('button').find((b) => b.textContent?.trim() === name) as HTMLButtonElement;

const row = (n: number) => ({
	id: `${n}`.repeat(40),
	short: `${n}`.repeat(7),
	summary: `Commit ${n}`,
	authorName: 'Ada',
	time: 100 + n,
	paths: [`f${n}.rs`]
});

const preview = (overrides = {}) => ({
	rows: [{ id: row(1).id, short: row(1).short, summary: 'Commit 1', absorbed: [], reworded: false, mayConflict: false }],
	dropped: [],
	refusal: null,
	emptiesTheBranch: false,
	...overrides
});

beforeEach(() => {
	vi.clearAllMocks();
	rebase.clear();
	repoControl.setInfo({
		path: '/repos/fixture',
		name: 'fixture',
		bare: false,
		lastFetched: null,
		head: { branch: 'topic', detached: false, id: 'c'.repeat(40), short: 'ccccccc' }
	});
	vi.mocked(api.rebaseTodo).mockResolvedValue({
		upstream: 'b'.repeat(40),
		upstreamShort: 'bbbbbbb',
		rows: [row(1), row(2)],
		truncated: false
	});
	vi.mocked(api.rebasePreview).mockResolvedValue(preview());
});

afterEach(() => {
	view?.destroy();
	vi.restoreAllMocks();
});

async function planned() {
	rebase.upstream = 'main';
	await rebase.load();
}

/** FEAT-106: the Rebase screen, drawn after Merger. */
describe('rebase route', () => {
	it('asks for a branch to replay onto before anything else', () => {
		view = render(Page, {});

		expect(view.text()).toContain('Choose a branch to replay onto');
		expect(view.text()).toContain('Dry run · nothing written yet');
		expect(button('Rebase now')).toBeUndefined();
	});

	it('plans as soon as a branch is picked', async () => {
		view = render(Page, {});

		click(view.get('.picker'));
		const main = [...document.querySelectorAll('[role="menuitem"]')].find(
			(item) => item.textContent?.trim() === 'main'
		) as HTMLElement;
		click(main);
		await vi.waitFor(() => expect(rebase.loaded).toBe(true));
		flushSync();

		expect(api.rebaseTodo).toHaveBeenCalledWith('main');
		expect(view.text()).toContain('2 commits since bbbbbbb');
	});

	it('shows the result, the plan and the history, then rebases', async () => {
		await planned();
		view = render(Page, {});

		expect(view.text()).toContain('2 commits of topic are replayed on top of main');
		expect(view.text()).toContain('Nothing looks likely to conflict');
		expect(view.all('.plan .row')).toHaveLength(2);
		expect(view.text()).toContain('onto main · bbbbbbb');

		click(button('Rebase now'));
		expect(actions.runRebase).toHaveBeenCalledWith('topic', 2, 0);
	});

	it('says how many may conflict, and counts what the plan folds and drops', async () => {
		vi.mocked(api.rebasePreview).mockResolvedValue(
			preview({ rows: [{ ...preview().rows[0], mayConflict: true }], dropped: [row(2).id] })
		);
		await planned();
		await rebase.setAction(row(2).id, 'drop');
		view = render(Page, {});
		flushSync();

		expect(view.text()).toContain('1 commit may stop on a conflict');
		expect(view.text()).toContain('1 dropped');
		expect(button('Reset plan')).toBeDefined();
	});

	it('will not rebase a plan the preview refuses, and says why', async () => {
		vi.mocked(api.rebasePreview).mockResolvedValue(preview({ refusal: 'Cannot squash the first commit' }));
		await planned();
		view = render(Page, {});

		expect(view.text()).toContain('This plan cannot run');
		expect(button('Rebase now').disabled).toBe(true);
		expect(button('Rebase now').title).toBe('Cannot squash the first commit');
	});

	it('says when there is nothing to replay', async () => {
		vi.mocked(api.rebaseTodo).mockResolvedValue({ upstream: 'b', upstreamShort: 'b', rows: [], truncated: false });
		await planned();
		view = render(Page, {});

		expect(view.text()).toContain('Nothing to replay: topic has no commits that main lacks');
	});

	it('shows the read failure', async () => {
		vi.mocked(api.rebaseTodo).mockRejectedValueOnce(new Error('unknown revision'));
		await planned();
		view = render(Page, {});

		expect(view.text()).toContain('unknown revision');
	});

	it.each([null, { step: 2, total: 3, branch: 'topic' }])('shows how far a running rebase has got (%j)', (progress) => {
		vi.spyOn(rebase, 'running', 'get').mockReturnValue(true);
		vi.spyOn(rebase, 'progress', 'get').mockReturnValue(progress as never);
		view = render(Page, {});

		expect(view.get('[role="progressbar"]').getAttribute('aria-valuenow')).toBe(String(progress?.step ?? 0));
		expect(view.text()).toContain(progress ? 'Replaying commit 2 of 3' : 'Starting the rebase');
		expect(view.text()).toContain(progress ? 'Replaying 2 of 3' : 'Starting');
	});

	it('offers the ways on when git stops', () => {
		vi.spyOn(rebase, 'stopped', 'get').mockReturnValue(true);
		vi.spyOn(rebase, 'progress', 'get').mockReturnValue({ step: 1, total: 3, branch: 'topic' } as never);
		vi.spyOn(rebase, 'runError', 'get').mockReturnValue('Resolve a.rs');
		view = render(Page, {});

		expect(view.text()).toContain('Stopped at commit 1 of 3');
		expect(view.text()).toContain('Resolve a.rs');
		click(button('Resolve conflicts'));
		expect(goto).toHaveBeenCalledWith('/conflicts');
		click(button('Continue'));
		expect(actions.continueRebase).toHaveBeenCalled();
		click(button('Skip this commit'));
		expect(actions.skipCommit).toHaveBeenCalled();
		click(button('Abort'));
		expect(actions.abortRebase).toHaveBeenCalled();
	});

	it('reports a truncated plan instead of claiming the whole history is shown', async () => {
		vi.mocked(api.rebaseTodo).mockResolvedValue({ upstream: 'b', upstreamShort: 'b', rows: [row(1)], truncated: true });
		await planned();
		view = render(Page, {});

		expect(view.text()).toContain('Only the first 1 commits are shown');
	});

	it('will not rebase a detached HEAD', async () => {
		repoControl.setInfo({
			path: '/repos/fixture',
			name: 'fixture',
			bare: false,
			lastFetched: null,
			head: { branch: null, detached: true, id: 'c'.repeat(40), short: 'ccccccc' }
		});
		await planned();
		view = render(Page, {});

		expect(button('Rebase now').disabled).toBe(true);
		expect(button('Rebase now').title).toBe('HEAD is not on a branch');
	});
});
