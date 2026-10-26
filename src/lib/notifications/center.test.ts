// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The notification center (FEAT-142): how the drawer slices what was kept, what
 * each source is turned into, and the drawer itself.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import { dayLabel, groups, timeLabel, unreadCount, visible, type Entry } from './center';
import { fromFarmEvent, fromNews, fromNotice, fromRelease, fromRuns } from './feeds';
import { KINDS, styleOf } from './kinds';
import type { Farm } from '$lib/farm/types';
import type { Run } from '$lib/pipelines/types';

vi.mock('$app/navigation', () => ({ goto: vi.fn(async () => {}) }));

const NOW = new Date(2026, 9, 18, 15, 0).getTime();
const HOUR = 60 * 60 * 1000;

function entry(id: string, over: Partial<Entry> = {}): Entry {
	return {
		id,
		repository: '/r/spagitty',
		repositoryName: 'spagitty',
		category: 'farm',
		kind: 'taskStuck',
		title: `title ${id}`,
		body: '',
		target: { kind: 'route', route: '/farm' },
		createdMs: NOW - HOUR,
		readMs: null,
		...over
	};
}

describe('the drawer’s slices', () => {
	const kept = [
		entry('a', { category: 'pullRequests', kind: 'reviewRequested', title: 'Review requested' }),
		entry('b', { readMs: NOW, title: 'Fetch failed', category: 'git', kind: 'fetchFailed', repository: '/r/quiblo', repositoryName: 'quiblo' }),
		entry('c', { title: 'T-09 is stuck after 3 tries', body: 'libsecret-1 is missing' })
	];

	it('keeps only the unread under Unread, and one category under its chip', () => {
		const ids = (filter: Parameters<typeof visible>[1]['filter']) =>
			visible(kept, { filter, repository: null, search: '' }).map((e) => e.id);
		expect(ids('all')).toEqual(['a', 'b', 'c']);
		expect(ids('unread')).toEqual(['a', 'c']);
		expect(ids('git')).toEqual(['b']);
		expect(ids('pullRequests')).toEqual(['a']);
	});

	it('keeps this repository’s alone when asked, and searches every word', () => {
		expect(visible(kept, { filter: 'all', repository: '/r/quiblo', search: '' }).map((e) => e.id)).toEqual(['b']);
		expect(visible(kept, { filter: 'all', repository: null, search: 'libsecret stuck' }).map((e) => e.id)).toEqual(['c']);
		expect(visible(kept, { filter: 'all', repository: null, search: 'quiblo' }).map((e) => e.id)).toEqual(['b']);
	});

	it('counts the unread, everywhere or in one repository', () => {
		expect(unreadCount(kept)).toBe(2);
		expect(unreadCount(kept, '/r/quiblo')).toBe(0);
	});

	it('names the days Today, Yesterday, then the weekday and the date', () => {
		expect(dayLabel(NOW - HOUR, NOW)).toBe('Today');
		expect(dayLabel(NOW - 20 * HOUR, NOW)).toBe('Yesterday');
		expect(dayLabel(new Date(2026, 9, 14, 20, 15).getTime(), NOW)).toBe('Wednesday, Oct 14');
	});

	it('groups newest first, one group per day', () => {
		const days = groups(
			[
				entry('old', { createdMs: new Date(2026, 9, 14, 9, 0).getTime() }),
				entry('today', { createdMs: NOW - 6 * 60_000 }),
				entry('yesterday', { createdMs: NOW - 22 * HOUR })
			],
			NOW
		);
		expect(days.map((d) => [d.label, d.entries.map((e) => e.id)])).toEqual([
			['Today', ['today']],
			['Yesterday', ['yesterday']],
			['Wednesday, Oct 14', ['old']]
		]);
	});

	it('says minutes and hours today, and the clock before that', () => {
		expect(timeLabel(NOW - 6 * 60_000, NOW)).toBe('6 min');
		expect(timeLabel(NOW - 2 * HOUR, NOW)).toBe('2 h');
		expect(timeLabel(new Date(2026, 9, 17, 18, 40).getTime(), NOW)).toBe('18:40');
	});

	it('draws a kind a later build wrote, quietly, rather than not at all', () => {
		expect(styleOf('somethingNew').action).toBe('Open');
		expect(styleOf('mergeReady')).toBe(KINDS.mergeReady);
	});
});

describe('what each source is turned into', () => {
	it('keeps the watcher’s headline verbatim and links the pull request', () => {
		const kept = fromNews({
			kind: 'reviewRequested',
			title: 'Review requested',
			body: 'Issues: a GitLab provider · spa-git-ty/spagitty#66',
			pr: { url: 'https://github.com/spa-git-ty/spagitty/pull/66' }
		});
		expect(kept).toMatchObject({ category: 'pullRequests', kind: 'reviewRequested', title: 'Review requested' });
		expect(kept.target).toEqual({ kind: 'url', url: 'https://github.com/spa-git-ty/spagitty/pull/66' });
	});

	it('keeps a fetch, a pull or a push that failed, in git’s words, and nothing else', () => {
		expect(fromNotice({ tone: 'error', title: 'Could not fetch', detail: 'origin: could not resolve host github.com\nmore' })).toMatchObject({
			kind: 'fetchFailed',
			title: 'Fetch failed',
			body: 'origin: could not resolve host github.com'
		});
		expect(fromNotice({ tone: 'error', title: 'Could not pull — your changes are in the stash', detail: 'x' })?.kind).toBe('pullFailed');
		expect(fromNotice({ tone: 'error', title: 'Could not push', detail: null })?.kind).toBe('pushFailed');
		expect(fromNotice({ tone: 'ok', title: 'Fetched', detail: null })).toBeNull();
		expect(fromNotice({ tone: 'error', title: 'Could not save the rules', detail: 'x' })).toBeNull();
	});

	it('keeps a published release with its files', () => {
		expect(fromRelease('v3.2.0', 3, null)).toMatchObject({ kind: 'releasePublished', title: 'v3.2.0 published', body: '3 files' });
	});

	const farm = {
		autonomy: 'semiAuto',
		status: 'running',
		goal: { title: 'Gitea support for pull requests' },
		tasks: [
			{ id: 'T-02', title: 'gitea.rs: list, create and merge', status: 'review', attempts: 1 },
			{ id: 'T-09', title: 'Store the token in the system keyring', status: 'failed', attempts: 3 },
			{ id: 'T-03', title: 'done', status: 'done', attempts: 1 }
		]
	} as unknown as Farm;

	it('says a task is ready to merge only where a person merges', () => {
		const ready = fromFarmEvent({ kind: 'reviewCompleted', task: 'T-02', reviewer: 'codex', approved: true, summary: '' }, farm, 'running');
		expect(ready).toMatchObject({ kind: 'mergeReady', title: 'T-02 is ready to merge' });
		expect(ready?.target).toEqual({ kind: 'route', route: '/farm?task=T-02' });
		const auto = { ...farm, autonomy: 'auto' } as Farm;
		expect(fromFarmEvent({ kind: 'reviewCompleted', task: 'T-02', reviewer: 'codex', approved: true, summary: '' }, auto, 'running')).toBeNull();
		expect(fromFarmEvent({ kind: 'reviewCompleted', task: 'T-02', reviewer: 'codex', approved: false, summary: '' }, farm, 'running')).toBeNull();
	});

	it('says a task is stuck with its tries', () => {
		expect(fromFarmEvent({ kind: 'taskStatusChanged', task: 'T-09', status: 'failed', note: 'libsecret-1 is missing' }, farm, 'running')).toMatchObject({
			kind: 'taskStuck',
			title: 'T-09 is stuck after 3 tries',
			body: 'Store the token in the system keyring · libsecret-1 is missing'
		});
		expect(fromFarmEvent({ kind: 'taskStatusChanged', task: 'T-02', status: 'running', note: null }, farm, 'running')).toBeNull();
	});

	it('says the farm stopped or finished only when its status changes', () => {
		const stopped = fromFarmEvent({ kind: 'farmStatusChanged', status: 'paused' }, farm, 'running');
		expect(stopped).toMatchObject({ kind: 'farmStopped', body: 'Gitea support for pull requests · 1 of 3 merged' });
		expect(fromFarmEvent({ kind: 'farmStatusChanged', status: 'paused' }, farm, 'paused')).toBeNull();
		expect(fromFarmEvent({ kind: 'farmStatusChanged', status: 'paused' }, farm, null)).toBeNull();
		expect(fromFarmEvent({ kind: 'farmStatusChanged', status: 'completed' }, farm, 'running')?.kind).toBe('farmFinished');
	});

	function run(id: number, result: string, ended: number, over: Partial<Run> = {}): Run {
		return { id, name: 'check', branch: 'feature/hooks-tab', commit: '', event: 'push', state: 'completed', result, url: `https://ci/${id}`, started: ended - 60, ended, duration: 60, ...over };
	}

	it('says nothing on the first look, then says a failure and a recovery once', () => {
		const first = fromRuns('feature/hooks-tab', [run(1, 'failure', 10)], undefined);
		expect(first).toEqual({ now: 'failed', said: null });

		const broke = fromRuns('feature/hooks-tab', [run(1, 'success', 10), run(2, 'failure', 20)], 'passed');
		expect(broke.now).toBe('failed');
		expect(broke.said).toMatchObject({ kind: 'runFailed', title: 'Checks failed', body: 'check on feature/hooks-tab' });
		expect(broke.said?.target).toEqual({ kind: 'url', url: 'https://ci/2' });

		expect(fromRuns('feature/hooks-tab', [run(2, 'failure', 20)], 'failed').said).toBeNull();
		expect(fromRuns('feature/hooks-tab', [run(3, 'success', 30)], 'failed').said?.kind).toBe('runPassing');
	});

	it('reads no verdict from a run that is still going or was cancelled', () => {
		const going = fromRuns('feature/hooks-tab', [run(4, '', 0, { state: 'running' }), run(5, 'cancelled', 40)], 'passed');
		expect(going).toEqual({ now: 'passed', said: null });
	});
});

describe('the drawer', () => {
	afterEach(async () => {
		const { center } = await import('./center.svelte');
		center.reset();
	});

	it('lists what was kept by day, marks one read, and closes on Escape', async () => {
		const { center } = await import('./center.svelte');
		const { default: NotificationCenter } = await import('./NotificationCenter.svelte');
		center.reset([
			entry('a', { title: 'T-02 is ready to merge', kind: 'mergeReady', createdMs: Date.now() - 60_000 }),
			entry('b', { title: 'Fetch failed', kind: 'fetchFailed', category: 'git', createdMs: Date.now() - 2 * 24 * HOUR, readMs: 1 })
		]);
		center.show();
		const { target: container } = render(NotificationCenter, {});
		flushSync();

		expect(container.querySelector('[role="dialog"]')).not.toBeNull();
		expect(container.textContent).toContain('1 new');
		expect([...container.querySelectorAll('.day-label')].map((d) => d.textContent)).toEqual(['Today', expect.any(String)]);

		const markRead = [...container.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Mark read');
		click(markRead!);
		flushSync();
		expect(center.unread).toBe(0);
		expect(container.textContent).toContain('all read');

		window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
		flushSync();
		expect(center.open).toBe(false);
		expect(container.querySelector('[role="dialog"]')).toBeNull();
		// The first test to import the drawer compiles it, which takes seconds
		// while the rest of the suite runs beside it: past the 5 s default.
	}, 15_000);

	it('says so when there is nothing under a chip', async () => {
		const { center } = await import('./center.svelte');
		const { default: NotificationCenter } = await import('./NotificationCenter.svelte');
		center.reset([entry('a', { readMs: 5 })]);
		center.show();
		const { target: container } = render(NotificationCenter, {});
		flushSync();
		const unreadChip = [...container.querySelectorAll('button')].find((b) => b.textContent?.trim().startsWith('Unread'));
		click(unreadChip!);
		flushSync();
		expect(container.textContent).toContain('Nothing here. You’re caught up.');
	});

	it('clears all, read or not, once it is agreed to (FEAT-159)', async () => {
		const { center } = await import('./center.svelte');
		const { dialog } = await import('$lib/ui/dialog.svelte');
		const { default: NotificationCenter } = await import('./NotificationCenter.svelte');
		center.reset([entry('a'), entry('b', { readMs: 5, repository: '/r/other' })]);
		center.show();
		const { target: container } = render(NotificationCenter, {});
		flushSync();

		const clearAll = () => [...container.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Clear all')!;
		click(clearAll());
		await Promise.resolve();
		expect(dialog.question?.title).toBe('Clear all notifications');
		expect(dialog.question?.body).toContain('All 2 notifications, from every repository');
		dialog.dismiss();
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(center.entries).toHaveLength(2);

		click(clearAll());
		await Promise.resolve();
		dialog.accept();
		await new Promise((resolve) => setTimeout(resolve, 0));
		flushSync();
		expect(center.entries).toHaveLength(0);
		expect((clearAll() as HTMLButtonElement).disabled).toBe(true);
	});

	it('clears one repository and keeps the others', async () => {
		const { center } = await import('./center.svelte');
		center.reset([entry('a'), entry('b', { repository: '/r/other' })]);
		expect(await center.clearAll('/r/spagitty')).toBe(1);
		expect(center.entries.map((e) => e.id)).toEqual(['b']);
	});
});

describe('which pull request a host page is', () => {
	it('reads the number only from a page in the open repository', async () => {
		const { numberIn } = await import('./act');
		const here = { host: 'github.com', owner: 'spa-git-ty', name: 'spagitty' };
		expect(numberIn('https://github.com/spa-git-ty/spagitty/pull/66', here)).toBe(66);
		expect(numberIn('https://gitlab.example.com/a/b/-/merge_requests/12', { host: 'gitlab.example.com', owner: 'a', name: 'b' })).toBe(12);
		expect(numberIn('https://github.com/someone/else/pull/66', here)).toBeNull();
		expect(numberIn('https://github.com/spa-git-ty/spagitty/pull/66', null)).toBeNull();
	});
});
