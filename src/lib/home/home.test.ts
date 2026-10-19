// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Home (FEAT-140): what Needs you lists and in which order, the arithmetic
 * behind the cards, and the screen mounted with and without a repository.
 */

import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Task, TaskDetail } from '$lib/farm/types';
import type { Issue } from '$lib/issues/types';
import type { Run } from '$lib/pipelines/types';
import type { PullRequest, RepoInfo } from '$lib/types';
import { age, counts, filtered, needs, summary } from './needs';
import { branchTag, driftBar, heatmap, level, nextVersion, pullRequestOrder, pullRequestState, unreleasedLine, workflows } from './shape';
import type { RecentBranch } from './types';

vi.mock('$app/navigation', () => ({ goto: vi.fn(async () => {}) }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://x/home') } }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/graph/store.svelte', async () => await import('../../testing/graph-store.svelte'));

const NOW = new Date(2026, 9, 18, 15, 0).getTime();
const MIN = 60_000;

function task(id: string, over: Partial<Task> = {}): Task {
	return {
		id,
		title: `task ${id}`,
		description: '',
		status: 'running',
		kind: 'general',
		priority: 'normal',
		dependsOn: [],
		parent: null,
		origin: { kind: 'person' },
		assignedAgent: null,
		implementedBy: null,
		allowedPaths: [],
		acceptanceCriteria: [],
		verification: [],
		verificationOverrides: false,
		branch: null,
		worktree: null,
		attempts: 1,
		note: null,
		createdMs: 0,
		updatedMs: NOW - 10 * MIN,
		...over
	};
}

function pr(number: number, over: Partial<PullRequest> = {}): PullRequest {
	return {
		id: String(number),
		number,
		title: `pull request ${number}`,
		body: '',
		authorName: 'nour',
		updated: (NOW - 12 * MIN) / 1000,
		sourceBranch: `feature/${number}`,
		targetBranch: 'main',
		draft: false,
		review: 'awaitingReview',
		checks: 'passing',
		needsYou: false,
		needsYouBecause: null,
		changedFiles: 14,
		added: 612,
		removed: 40,
		mergeable: true,
		headSha: '',
		reviewRequested: false,
		openThreads: 0,
		resolvedThreads: 0,
		repliesToYou: 0,
		repository: null,
		...over
	};
}

function run(id: number, over: Partial<Run> = {}): Run {
	return { id, name: 'check', branch: 'feature/hooks-tab', commit: '', event: 'push', state: 'completed', result: 'success', url: `https://ci/${id}`, started: id * 100, ended: id * 100 + 60, duration: 60, ...over };
}

const ready: TaskDetail = {
	task: task('T-02', { status: 'review' }),
	verification: { results: [], passed: true, unverified: false },
	review: { decision: 'approve', summary: '', issues: [] },
	handoff: null,
	runs: [],
	stats: { commits: 1, files: [{ path: 'a', added: 412, removed: 18, binary: false }] }
};

const issue = { id: 'i151', number: 151, title: 'Crash when a repository has no commits', body: '', state: 'open', author: 'salma', created: new Date(NOW - 120 * MIN).toISOString(), updated: new Date(NOW - 120 * MIN).toISOString(), url: 'https://host/151', replies: 0, labels: [{ name: 'bug', color: '' }], assignees: ['me'], milestone: null, locked: false, branchSuggestion: '' } as Issue;

describe('Needs you', () => {
	const rows = needs({
		farm: {
			tasks: [
				task('T-02', { status: 'review', title: 'gitea.rs: list, create and merge', updatedMs: NOW - 6 * MIN }),
				task('T-09', { status: 'failed', title: 'Store the token in the system keyring', attempts: 3, note: 'libsecret-1 is missing', updatedMs: NOW - 9 * MIN }),
				task('T-04', { status: 'running' })
			],
			details: { 'T-02': ready },
			runs: [{ id: 'r', task: 'T-02', agent: 'codex', phase: 'review', outcome: { state: 'completed', exitCode: 0 }, command: [], startedMs: 0, endedMs: 1, logFile: null, lastOutputMs: null }]
		},
		pullRequests: [
			pr(63, { authorName: 'me', review: 'changesRequested', openThreads: 3, updated: (NOW - 60 * MIN) / 1000 }),
			pr(66, { reviewRequested: true }),
			pr(70, { reviewRequested: true, draft: true }),
			pr(71)
		],
		me: 'me',
		failedRuns: [run(4, { result: 'failure', ended: (NOW - 21 * MIN) / 1000 })],
		assigned: [issue]
	});

	it('lists the six kinds in the handoff’s order', () => {
		expect(rows.map((r) => r.kind)).toEqual([
			'mergeReady',
			'reviewRequested',
			'taskStuck',
			'runFailed',
			'changesRequested',
			'assigned'
		]);
	});

	it('words each row the way the handoff does', () => {
		expect(rows[0]).toMatchObject({ title: 'T-02 gitea.rs: list, create and merge', meta: 'Farm · checks passed · codex approved · +412 −18' });
		expect(rows[1]).toMatchObject({ title: '#66 pull request 66', meta: 'nour asked you · 14 files · +612 −40' });
		expect(rows[2]).toMatchObject({ title: 'T-09 Store the token in the system keyring', meta: 'Farm · failed 3 of 3 · libsecret-1 is missing' });
		expect(rows[3]).toMatchObject({ title: 'feature/hooks-tab', meta: 'Your branch · check failed' });
		expect(rows[4]).toMatchObject({ title: '#63 pull request 63', meta: 'Your pull request · 3 open threads' });
		expect(rows[5]).toMatchObject({ title: '#151 Crash when a repository has no commits', meta: 'Issue · bug · opened by salma' });
	});

	it('leaves out a draft, a pull request that asks nothing, and a task that is only working', () => {
		const titles = rows.map((r) => r.title);
		expect(titles).not.toContain('#70 pull request 70');
		expect(titles).not.toContain('#71 pull request 71');
		expect(titles.some((t) => t.startsWith('T-04'))).toBe(false);
	});

	it('puts the longest waiting first within a kind', () => {
		const two = needs({ farm: null, pullRequests: [pr(1, { reviewRequested: true, updated: 200 }), pr(2, { reviewRequested: true, updated: 100 })], me: null, failedRuns: [], assigned: [] });
		expect(two.map((r) => r.title)).toEqual(['#2 pull request 2', '#1 pull request 1']);
	});

	it('filters by chip and counts each one', () => {
		expect(filtered(rows, 'reviews').map((r) => r.kind)).toEqual(['reviewRequested', 'changesRequested']);
		expect(filtered(rows, 'failing').map((r) => r.kind)).toEqual(['taskStuck', 'runFailed']);
		expect(filtered(rows, 'farm').map((r) => r.kind)).toEqual(['mergeReady', 'taskStuck']);
		expect(counts(rows)).toEqual({ all: 6, reviews: 2, failing: 2, farm: 2 });
	});

	it('says how many and how old the oldest is', () => {
		expect(summary(rows, NOW)).toBe('6 · oldest 2 h');
		expect(summary([], NOW)).toBe('nothing waiting');
		expect(age(NOW - 6 * MIN, NOW)).toBe('6 min');
		expect(age(NOW - 3 * 24 * 60 * MIN, NOW)).toBe('3 d');
	});
});

describe('the cards’ arithmetic', () => {
	it('draws sixteen weeks of seven days, with today in the last column', () => {
		const map = heatmap([NOW / 1000, NOW / 1000 - 60, (NOW - 7 * 24 * 60 * MIN) / 1000], NOW);
		expect(map.weeks).toHaveLength(16);
		expect(map.weeks.every((week) => week.length === 7)).toBe(true);
		expect(map.total).toBe(3);
		const last = map.weeks[15].find((d) => d.count === 2);
		expect(last?.level).toBe(4);
		expect(map.busiest).toBe('Sundays');
	});

	it('steps the heat in four quarters of the busiest day', () => {
		expect([0, 1, 2, 3, 4].map((n) => level(n, 4))).toEqual([0, 1, 2, 3, 4]);
		expect(level(1, 100)).toBe(1);
	});

	it('proposes the next version by the changelog’s own rule', () => {
		expect(nextVersion('v3.1.1', { total: 2, sections: [['Fixed', 2]] })).toBe('v3.1.2');
		expect(nextVersion('v3.1.1', { total: 3, sections: [['Added', 2], ['Fixed', 1]] })).toBe('v3.2.0');
		expect(nextVersion('v3.1.1', { total: 1, sections: [['Removed', 1]] })).toBe('v4.0.0');
		expect(nextVersion('v0.8.1', { total: 1, sections: [['Removed', 1]] })).toBe('v0.9.0');
		expect(nextVersion(null, null)).toBe('v0.1.0');
		expect(nextVersion('nightly', null)).toBeNull();
		expect(unreleasedLine({ total: 14, sections: [['Added', 8], ['Fixed', 5], ['Removed', 1]] })).toBe('8 added · 5 fixed · 1 removed');
	});

	const branch = (name: string, over: Partial<RecentBranch> = {}): RecentBranch => ({ name, authorName: 'you', time: 0, ahead: 3, behind: 0, current: false, ...over });

	it('tags a branch with what it is', () => {
		const failing = new Set(['feature/hooks-tab']);
		expect(branchTag(branch('feature/hooks-tab'), [], failing, 'main')).toEqual({ text: 'checks failing', colour: 'var(--danger)' });
		expect(branchTag(branch('fix/blame'), [pr(63, { sourceBranch: 'fix/blame' })], failing, 'main')?.text).toBe('PR #63');
		expect(branchTag(branch('spagitty-farm/TASK-0004/claude'), [], failing, 'main')?.text).toBe('farm · Claude Code');
		expect(branchTag(branch('release/0.8'), [], failing, 'main')?.text).toBe('release');
		expect(branchTag(branch('idea'), [], failing, 'main')).toBeNull();
	});

	it('scales the drift bar against the widest drift, with a floor so one commit shows', () => {
		expect(driftBar(branch('a', { ahead: 18, behind: 5 }), 18)).toEqual({ ahead: 100, behind: 28 });
		expect(driftBar(branch('b', { ahead: 1, behind: 0 }), 18)).toEqual({ ahead: 8, behind: 0 });
	});

	it('orders pull requests yours and waiting on you first, and names their state', () => {
		const order = pullRequestOrder([pr(1, { updated: 9 }), pr(2, { authorName: 'me', updated: 1 }), pr(3, { reviewRequested: true, updated: 2 })], 'me');
		expect(order.map((p) => p.number)).toEqual([3, 2, 1]);
		expect(pullRequestState(pr(1, { draft: true }), 'me').text).toBe('Draft');
		expect(pullRequestState(pr(1, { reviewRequested: true }), 'me').text).toBe('Your review');
		expect(pullRequestState(pr(1, { review: 'approved' }), 'me').text).toBe('Approved');
		expect(pullRequestState(pr(1, { review: 'changesRequested' }), 'me').text).toBe('Changes requested');
	});

	it('keeps the last fourteen runs of each workflow, newest on the right', () => {
		const runs = Array.from({ length: 20 }, (_, i) => run(i + 1, { name: i % 2 ? 'release' : 'check', result: i === 18 ? 'failure' : 'success' }));
		runs.push(run(30, { name: 'check', state: 'running', result: '' }));
		const [check, release] = workflows(runs);
		expect(check.name).toBe('check');
		expect(check.runs).toHaveLength(11);
		expect(check.runs.at(-1)?.state).toBe('running');
		expect(check.runs.at(-2)?.state).toBe('failed');
		expect(release.runs.every((r) => r.state === 'ok')).toBe(true);
	});
});

// The first import of the screen pulls in most of the application.
describe('the screen', { timeout: 30_000 }, () => {
	afterEach(async () => {
		const { control } = await import('../../testing/repo-store.svelte');
		control.reset();
	});

	const info: RepoInfo = { path: '/repos/spagitty', name: 'spagitty', bare: false, lastFetched: null, head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' } };

	it('asks for a repository when none is open', async () => {
		const { render } = await import('../../testing/mount');
		const { default: HomeScreen } = await import('./HomeScreen.svelte');
		const view = render(HomeScreen, {});
		expect(view.text()).toContain('Open a repository');
		view.destroy();
	});

	it('draws the header and every card from what was read', async () => {
		const { control } = await import('../../testing/repo-store.svelte');
		const { render, flushSync } = await import('../../testing/mount');
		const { home } = await import('./store.svelte');
		const { default: HomeScreen } = await import('./HomeScreen.svelte');
		control.setInfo(info);
		const view = render(HomeScreen, {});
		home.present({
			overview: {
				remote: 'github.com/spa-git-ty/spagitty',
				licence: 'GPL-3.0',
				version: 'v3.1.1',
				versionTime: NOW / 1000,
				upstream: { upstream: 'origin/main', ahead: 2, behind: 0 },
				defaultBranch: 'main',
				branches: [{ name: 'main', authorName: 'you', time: NOW / 1000, ahead: 2, behind: 0, current: true }],
				branchCount: 23,
				commitTimes: [NOW / 1000],
				unreleased: { total: 3, sections: [['Added', 2], ['Fixed', 1]] }
			},
			changed: [{ path: 'src/lib/home/HomeScreen.svelte', status: 'modified' }],
			assigned: [issue]
		});
		flushSync();

		const text = view.text();
		expect(text).toContain('spagitty');
		expect(text).toContain('github.com/spa-git-ty/spagitty · GPL-3.0 · v3.1.1');
		expect(text).toContain('↑2');
		expect(text).toContain('Needs you');
		expect(text).toContain('Assigned to you');
		expect(text).toContain('All 23');
		expect(text).toContain('src/lib/home/HomeScreen.svelte');
		expect(text).toContain('3 entries under Unreleased');
		expect(text).toContain('Draft v3.2.0');
		expect(text).toContain('Start a farm');
		view.destroy();
	});

	it('draws the cards in the order Settings › Home chose, without the hidden ones (FEAT-148)', async () => {
		const { control } = await import('../../testing/repo-store.svelte');
		const { render } = await import('../../testing/mount');
		const { settings } = await import('$lib/settings/store.svelte');
		const { default: HomeScreen } = await import('./HomeScreen.svelte');
		control.setInfo(info);
		settings.settings.home = { hidden: ['heat', 'pipelines'], order: ['releases', 'farm'] };
		const view = render(HomeScreen, {});

		const cards = view.all('.slot').map((slot) => slot.dataset.card);
		// A card the order does not name keeps its designed place: Needs you first.
		expect(cards.slice(0, 3)).toEqual(['needs', 'releases', 'farm']);
		expect(cards).not.toContain('heat');
		expect(cards).not.toContain('pipelines');
		expect(cards).toHaveLength(7);
		settings.settings.home = { hidden: [], order: [] };
		view.destroy();
	});
});

describe('the layout (FEAT-148)', () => {
	it('keeps the designed order until somebody moves a card', async () => {
		const { HOME_CARDS, inHomeOrder, moveCardBy, moveCardOnto, shownCards } = await import('./layout');

		expect(inHomeOrder([])).toBe(HOME_CARDS);
		expect(shownCards({ hidden: ['needs'], order: [] })[0].id).toBe('farm');

		const order = moveCardOnto('releases', 'needs', []);
		expect(order[0]).toBe('releases');
		// Moving it back is the designed order again, stored as nothing.
		expect(moveCardBy('releases', 8, order)).toEqual([]);
		expect(moveCardBy('needs', -1, [])).toEqual([]);
	});

	it('places a card the stored order does not name after the one it follows', async () => {
		const { inHomeOrder } = await import('./layout');
		// Written before `commits` existed, and naming a card from a later build.
		const ids = inHomeOrder(['later', 'requests', 'branches', 'needs']).map((card) => card.id);

		expect(ids.slice(0, 4)).toEqual(['requests', 'commits', 'heat', 'pipelines']);
		expect(ids.slice(-4)).toEqual(['branches', 'needs', 'farm', 'working']);
		expect(ids).toHaveLength(9);
	});
});
