// SPDX-License-Identifier: GPL-3.0-or-later

import type { Notice } from '$lib/ui/notice.svelte';
import type { Farm, FarmEvent, FarmStatus } from '$lib/farm/types';
import type { Run } from '$lib/pipelines/types';
import type { NewsKind } from './changes';
import type { NewEntry } from './center';

/**
 * What each source says, as a notification center entry (FEAT-142).
 *
 * Pure translations. The layout listens to the sources and hands what they say
 * here; whatever comes back is kept. A source that says something the center
 * has no kind for gets `null` and is said once, as before, and not kept.
 */

type Kept = Omit<NewEntry, 'repository' | 'repositoryName'>;

/** A pull request the watcher announced, with its headline verbatim. */
export function fromNews(news: {
	kind: NewsKind;
	title: string;
	body: string;
	pr: { url?: string };
}): Kept {
	return {
		category: 'pullRequests',
		kind: news.kind,
		title: news.title,
		body: news.body,
		target: news.pr.url ? { kind: 'url', url: news.pr.url } : { kind: 'route', route: '/review' }
	};
}

const GIT_FAILURES: { prefix: string; kind: string; title: string }[] = [
	{ prefix: 'Could not fetch', kind: 'fetchFailed', title: 'Fetch failed' },
	{ prefix: 'Could not pull', kind: 'pullFailed', title: 'Pull failed' },
	{ prefix: 'Could not push', kind: 'pushFailed', title: 'Push failed' }
];

/**
 * A fetch, a pull or a push that failed, from the notice that said so.
 *
 * Read off the notice's headline, because that is the one place every one of
 * them already goes through — the toolbar, the palette, Merger and a pull
 * that stashed first all say `Could not pull …` — and git's own words are its
 * detail.
 */
export function fromNotice(notice: Pick<Notice, 'tone' | 'title' | 'detail'>): Kept | null {
	if (notice.tone !== 'error') return null;
	const failure = GIT_FAILURES.find((f) => notice.title.startsWith(f.prefix));
	if (!failure) return null;
	return {
		category: 'git',
		kind: failure.kind,
		title: failure.title,
		body: (notice.detail ?? notice.title).split('\n')[0].trim(),
		target: { kind: 'route', route: '/' }
	};
}

/** A release published from the Releases screen. */
export function fromRelease(tag: string, files: number, url: string | null): Kept {
	return {
		category: 'git',
		kind: 'releasePublished',
		title: `${tag} published`,
		body: files === 1 ? '1 file' : `${files} files`,
		target: url ? { kind: 'url', url } : { kind: 'route', route: '/releases' }
	};
}

/** The farm statuses that mean it stopped by itself or by a person. */
const STOPPED: FarmStatus[] = ['paused', 'cancelled', 'failed'];

/**
 * What a farm event is worth keeping, given the farm it came from.
 *
 * `previous` is the farm's status before this event: the farm says its status
 * every time it is opened, and only a change is news. Ready to merge is said
 * only where a person merges — at Automatic and above the farm merges by
 * itself, and a notification would ask for a yes that is not needed.
 */
export function fromFarmEvent(
	event: FarmEvent,
	farm: Farm | null,
	previous: FarmStatus | null
): Kept | null {
	const title = (task: string) => farm?.tasks.find((t) => t.id === task)?.title ?? '';
	const route = (task: string) => ({ kind: 'route' as const, route: `/farm?task=${encodeURIComponent(task)}` });
	switch (event.kind) {
		case 'reviewCompleted': {
			if (!event.approved) return null;
			if (farm && (farm.autonomy === 'auto' || farm.autonomy === 'yolo')) return null;
			return {
				category: 'farm',
				kind: 'mergeReady',
				title: `${event.task} is ready to merge`,
				body: [title(event.task), `${event.reviewer} approved`].filter(Boolean).join(' · '),
				target: route(event.task)
			};
		}
		case 'taskStatusChanged': {
			// Failed outright, or blocked once its tries ran out: both wait on a person.
			if (event.status !== 'failed' && event.status !== 'blocked') return null;
			const tries = farm?.tasks.find((t) => t.id === event.task)?.attempts;
			return {
				category: 'farm',
				kind: 'taskStuck',
				title: tries ? `${event.task} is stuck after ${tries} ${tries === 1 ? 'try' : 'tries'}` : `${event.task} is stuck`,
				body: [title(event.task), event.note].filter(Boolean).join(' · '),
				target: route(event.task)
			};
		}
		case 'farmStatusChanged': {
			if (event.status === previous || previous === null) return null;
			const goal = farm?.goal.title ?? '';
			const tasks = (farm?.tasks ?? []).filter((t) => t.status !== 'cancelled');
			const merged = tasks.filter((t) => t.status === 'done').length;
			const progress = tasks.length ? `${merged} of ${tasks.length} merged` : '';
			if (event.status === 'completed') {
				return {
					category: 'farm',
					kind: 'farmFinished',
					title: 'Farm finished',
					body: [goal, progress].filter(Boolean).join(' · '),
					target: { kind: 'route', route: '/farm' }
				};
			}
			if (STOPPED.includes(event.status) && !STOPPED.includes(previous)) {
				return {
					category: 'farm',
					kind: 'farmStopped',
					title: 'Farm stopped',
					body: [goal, progress].filter(Boolean).join(' · '),
					target: { kind: 'route', route: '/farm' }
				};
			}
			return null;
		}
		default:
			return null;
	}
}

/** A finished run's result, in two words: did it pass. */
export type Outcome = 'passed' | 'failed';

export function outcomeOf(run: Run): Outcome | null {
	if (run.state !== 'completed') return null;
	if (['success'].includes(run.result)) return 'passed';
	if (['failure', 'failed', 'timed_out', 'startup_failure'].includes(run.result)) return 'failed';
	// Cancelled and skipped runs say nothing about whether the branch is broken.
	return null;
}

/**
 * The newest finished run's outcome on a branch, and what changed since the
 * last one seen.
 *
 * The first look has nothing to compare with and says nothing, as the pull
 * request watcher does: turning Spagitty on must not open with a failure that
 * was already old news.
 */
export function fromRuns(
	branch: string,
	runs: Run[],
	before: Outcome | null | undefined
): { now: Outcome | null; said: Kept | null } {
	const latest = runs
		.filter((run) => run.branch === branch || branch === '')
		.filter((run) => outcomeOf(run) !== null)
		.sort((a, b) => (b.ended || b.started) - (a.ended || a.started))[0];
	const now = latest ? outcomeOf(latest) : null;
	if (!latest || now === null || before === undefined || now === before) {
		return { now: now ?? before ?? null, said: null };
	}
	const target = latest.url
		? ({ kind: 'url', url: latest.url } as const)
		: ({ kind: 'route', route: '/pipelines/live' } as const);
	if (now === 'failed') {
		return {
			now,
			said: {
				category: 'checks',
				kind: 'runFailed',
				title: 'Checks failed',
				body: `${latest.name} on ${branch || latest.branch}`,
				target
			}
		};
	}
	if (before === 'failed') {
		return {
			now,
			said: {
				category: 'checks',
				kind: 'runPassing',
				title: 'Checks passing again',
				body: `${latest.name} on ${branch || latest.branch}`,
				target
			}
		};
	}
	return { now, said: null };
}
