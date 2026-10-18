// SPDX-License-Identifier: GPL-3.0-or-later

import { attention, readyToLand } from '$lib/farm/describe';
import type { AgentRun, Task, TaskDetail } from '$lib/farm/types';
import type { Issue } from '$lib/issues/types';
import { KINDS, type Kind } from '$lib/notifications/kinds';
import type { Run } from '$lib/pipelines/types';
import type { PullRequest } from '$lib/types';

/**
 * Home's Needs you (FEAT-140): one row per thing waiting on the person now.
 *
 * Pure, so the order and the wording are tested without a screen. A row leaves
 * once the thing is done — a task merged, a review left, a run green again —
 * because the sources stop reporting it; nothing here is marked read. The
 * notification center keeps the record of it having been said.
 */

/** The kinds Needs you lists, most urgent first: the handoff's table, in order. */
export const NEED_KINDS = [
	'mergeReady',
	'reviewRequested',
	'taskStuck',
	'runFailed',
	'changesRequested',
	'assigned'
] as const satisfies readonly Kind[];

export type NeedKind = (typeof NEED_KINDS)[number];

/** What a row is about, so its button knows what to act on. */
export type NeedSource =
	| { kind: 'task'; task: Task }
	| { kind: 'pr'; pr: PullRequest }
	| { kind: 'run'; run: Run }
	| { kind: 'issue'; issue: Issue };

export interface Need {
	key: string;
	kind: NeedKind;
	title: string;
	/** One line of context. */
	meta: string;
	/** When it started waiting, in milliseconds. */
	since: number;
	source: NeedSource;
}

export type NeedFilter = 'all' | 'reviews' | 'failing' | 'farm';

export const NEED_FILTERS: { id: NeedFilter; label: string; kinds: readonly NeedKind[] }[] = [
	{ id: 'all', label: 'All', kinds: NEED_KINDS },
	{ id: 'reviews', label: 'Reviews', kinds: ['reviewRequested', 'changesRequested'] },
	{ id: 'failing', label: 'Failing', kinds: ['taskStuck', 'runFailed'] },
	{ id: 'farm', label: 'Farm', kinds: ['mergeReady', 'taskStuck'] }
];

export interface NeedsInput {
	farm: {
		tasks: Task[];
		details: Record<string, TaskDetail>;
		runs: AgentRun[];
	} | null;
	pullRequests: PullRequest[];
	/** The person's name on the host, to tell their pull requests from others'. */
	me: string | null;
	/** Finished runs that failed, on the person's branches. */
	failedRuns: Run[];
	/** Open issues assigned to the person. */
	assigned: Issue[];
}

function size(added: number, removed: number): string {
	return `+${added} −${removed}`;
}

function plural(n: number, one: string, many = `${one}s`): string {
	return `${n} ${n === 1 ? one : many}`;
}

function farmNeeds(farm: NonNullable<NeedsInput['farm']>): Need[] {
	const out: Need[] = [];
	for (const task of farm.tasks) {
		if (!attention(task, farm.tasks, farm.details[task.id], farm.runs)) continue;
		const detail = farm.details[task.id];
		if (readyToLand(detail)) {
			const files = detail.stats?.files ?? [];
			const added = files.reduce((sum, f) => sum + f.added, 0);
			const removed = files.reduce((sum, f) => sum + f.removed, 0);
			const reviewer = farm.runs.filter((r) => r.task === task.id && r.phase === 'review').pop()?.agent;
			out.push({
				key: `task:${task.id}`,
				kind: 'mergeReady',
				title: `${task.id} ${task.title}`,
				meta: ['Farm', 'checks passed', reviewer ? `${reviewer} approved` : 'approved', files.length ? size(added, removed) : '']
					.filter(Boolean)
					.join(' · '),
				since: task.updatedMs,
				source: { kind: 'task', task }
			});
		} else if (task.status === 'failed' || task.status === 'blocked') {
			out.push({
				key: `task:${task.id}`,
				kind: 'taskStuck',
				title: `${task.id} ${task.title}`,
				meta: ['Farm', task.attempts > 1 ? `failed ${task.attempts} of ${task.attempts}` : '', task.note ?? '']
					.filter(Boolean)
					.join(' · '),
				since: task.updatedMs,
				source: { kind: 'task', task }
			});
		}
	}
	return out;
}

function pullRequestNeeds(prs: PullRequest[], me: string | null): Need[] {
	const out: Need[] = [];
	for (const pr of prs) {
		const mine = me !== null && pr.authorName === me;
		if (pr.reviewRequested && !pr.draft && !mine) {
			out.push({
				key: `pr:${pr.id}`,
				kind: 'reviewRequested',
				title: `#${pr.number} ${pr.title}`,
				meta: [`${pr.authorName} asked you`, plural(pr.changedFiles, 'file'), size(pr.added, pr.removed)].join(' · '),
				since: pr.updated * 1000,
				source: { kind: 'pr', pr }
			});
		} else if (mine && pr.review === 'changesRequested') {
			out.push({
				key: `pr:${pr.id}`,
				kind: 'changesRequested',
				title: `#${pr.number} ${pr.title}`,
				meta: ['Your pull request', pr.openThreads ? plural(pr.openThreads, 'open thread') : '']
					.filter(Boolean)
					.join(' · '),
				since: pr.updated * 1000,
				source: { kind: 'pr', pr }
			});
		}
	}
	return out;
}

function runNeeds(runs: Run[]): Need[] {
	// One row per branch: the newest failure says it, the older ones repeat it.
	const newest = new Map<string, Run>();
	for (const run of runs) {
		const seen = newest.get(run.branch);
		if (!seen || (run.ended || run.started) > (seen.ended || seen.started)) newest.set(run.branch, run);
	}
	return [...newest.values()].map((run) => ({
		key: `run:${run.branch}`,
		kind: 'runFailed' as const,
		title: run.branch,
		meta: `Your branch · ${run.name} failed`,
		since: (run.ended || run.started) * 1000,
		source: { kind: 'run' as const, run }
	}));
}

function issueNeeds(issues: Issue[]): Need[] {
	return issues.map((issue) => ({
		key: `issue:${issue.id}`,
		kind: 'assigned' as const,
		title: `#${issue.number} ${issue.title}`,
		meta: ['Issue', ...issue.labels.slice(0, 2).map((l) => l.name), issue.author ? `opened by ${issue.author}` : '']
			.filter(Boolean)
			.join(' · '),
		since: Date.parse(issue.updated) || Date.parse(issue.created) || 0,
		source: { kind: 'issue' as const, issue }
	}));
}

/** Every row, most urgent kind first, and the longest waiting first within one. */
export function needs(input: NeedsInput): Need[] {
	const all = [
		...(input.farm ? farmNeeds(input.farm) : []),
		...pullRequestNeeds(input.pullRequests, input.me),
		...runNeeds(input.failedRuns),
		...issueNeeds(input.assigned)
	];
	const rank = (kind: NeedKind) => NEED_KINDS.indexOf(kind);
	return all.sort((a, b) => rank(a.kind) - rank(b.kind) || a.since - b.since);
}

export function filtered(rows: Need[], filter: NeedFilter): Need[] {
	const kinds = NEED_FILTERS.find((f) => f.id === filter)?.kinds ?? NEED_KINDS;
	return rows.filter((row) => kinds.includes(row.kind));
}

/** How many a chip holds. */
export function counts(rows: Need[]): Record<NeedFilter, number> {
	return Object.fromEntries(NEED_FILTERS.map((f) => [f.id, filtered(rows, f.id).length])) as Record<NeedFilter, number>;
}

/** The style a row is drawn in, from the shared table. */
export function styleOfNeed(kind: NeedKind) {
	return KINDS[kind];
}

/** `6 · oldest 2 h` beside the heading. */
export function summary(rows: Need[], now: number): string {
	if (!rows.length) return 'nothing waiting';
	const oldest = Math.min(...rows.map((r) => r.since));
	return `${rows.length} · oldest ${age(oldest, now)}`;
}

/** `6 min`, `2 h`, `3 d`. */
export function age(since: number, now: number): string {
	const minutes = Math.max(0, Math.floor((now - since) / 60_000));
	if (minutes < 1) return 'now';
	if (minutes < 60) return `${minutes} min`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours} h`;
	return `${Math.floor(hours / 24)} d`;
}
