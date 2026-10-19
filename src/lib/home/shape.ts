// SPDX-License-Identifier: GPL-3.0-or-later

import type { Run } from '$lib/pipelines/types';
import type { PullRequest } from '$lib/types';
import type { RecentBranch, Unreleased } from './types';

/**
 * The arithmetic behind Home's cards (FEAT-140), kept out of the components so
 * it can be read and tested on its own.
 */

// ── Commits, last 16 weeks ───────────────────────────────────────────────

export const WEEKS = 16;

export interface Heatmap {
	/** Sixteen columns of seven days, oldest week first, Monday at the top. */
	weeks: { day: number; count: number; level: number }[][];
	total: number;
	/** `Tuesdays`, or null when nothing was committed. */
	busiest: string | null;
}

const WEEKDAYS = ['Mondays', 'Tuesdays', 'Wednesdays', 'Thursdays', 'Fridays', 'Saturdays', 'Sundays'];

function dayStart(ms: number): number {
	const day = new Date(ms);
	day.setHours(0, 0, 0, 0);
	return day.getTime();
}

/** Monday 0 … Sunday 6. */
function weekday(ms: number): number {
	return (new Date(ms).getDay() + 6) % 7;
}

/**
 * Five steps from none to the busiest day: none, then four quarters of the
 * busiest day's count. Relative, because "busy" means something different on a
 * repository with three commits a week and one with three hundred.
 */
export function level(count: number, busiest: number): number {
	if (count <= 0 || busiest <= 0) return 0;
	return Math.min(4, Math.max(1, Math.ceil((count / busiest) * 4)));
}

export function heatmap(commitTimes: number[], now: number): Heatmap {
	const today = dayStart(now);
	// The grid ends on this week's Sunday, so today sits in the last column.
	const lastSunday = today + (6 - weekday(today)) * 86_400_000;
	const first = lastSunday - (WEEKS * 7 - 1) * 86_400_000;
	const counts = new Map<number, number>();
	const perWeekday = new Array(7).fill(0);
	for (const seconds of commitTimes) {
		const day = dayStart(seconds * 1000);
		if (day < first || day > today) continue;
		counts.set(day, (counts.get(day) ?? 0) + 1);
		perWeekday[weekday(day)] += 1;
	}
	const busiestDay = Math.max(0, ...counts.values());
	const weeks: Heatmap['weeks'] = [];
	for (let w = 0; w < WEEKS; w++) {
		const column = [];
		for (let d = 0; d < 7; d++) {
			// Days are added by the calendar, not by 24 hours, so a change of
			// summer time does not shift a column by an hour into the next day.
			const date = new Date(first);
			date.setDate(date.getDate() + w * 7 + d);
			const day = dayStart(date.getTime());
			const count = day > today ? 0 : (counts.get(day) ?? 0);
			column.push({ day, count, level: level(count, busiestDay) });
		}
		weeks.push(column);
	}
	const total = [...counts.values()].reduce((sum, n) => sum + n, 0);
	const top = Math.max(...perWeekday);
	return { weeks, total, busiest: top > 0 ? WEEKDAYS[perWeekday.indexOf(top)] : null };
}

// ── Releases ─────────────────────────────────────────────────────────────

/**
 * The version the unreleased entries call for, by the changelog's own rule:
 * MAJOR is the only bump that may break what came before, MINOR adds and
 * PATCH fixes. A Removed section is a break.
 */
export function nextVersion(current: string | null, unreleased: Unreleased | null): string | null {
	const match = /^(v?)(\d+)\.(\d+)(?:\.(\d+))?$/.exec(current ?? '');
	if (!match) return current ? null : 'v0.1.0';
	const [, prefix, major, minor, patch] = match;
	const headings = (unreleased?.sections ?? []).map(([h]) => h.toLowerCase());
	const [a, b, c] = [Number(major), Number(minor), Number(patch ?? 0)];
	if (headings.includes('removed') && a > 0) return `${prefix}${a + 1}.0.0`;
	if (headings.some((h) => h === 'added' || h === 'changed' || h === 'deprecated' || h === 'removed')) {
		return `${prefix}${a}.${b + 1}.0`;
	}
	return `${prefix}${a}.${b}.${c + 1}`;
}

/** `8 added · 5 fixed · 1 removed`. */
export function unreleasedLine(unreleased: Unreleased): string {
	return unreleased.sections
		.filter(([heading]) => heading)
		.map(([heading, count]) => `${count} ${heading.toLowerCase()}`)
		.join(' · ');
}

// ── Branches ─────────────────────────────────────────────────────────────

/** The slugs `AgentProvider::slug` puts in a farm branch's name. */
const FARM_AGENTS: Record<string, string> = {
	claude: 'Claude Code',
	codex: 'Codex',
	cursor: 'Cursor',
	pi: 'Oh My Pi'
};

export interface BranchTag {
	text: string;
	/** A palette token. */
	colour: string;
}

/**
 * What a branch is, in two words: its pull request, the farm agent working on
 * it, checks failing, or nothing.
 */
export function branchTag(
	branch: RecentBranch,
	prs: PullRequest[],
	failing: Set<string>,
	defaultBranch: string | null
): BranchTag | null {
	if (failing.has(branch.name)) return { text: 'checks failing', colour: 'var(--danger)' };
	const pr = prs.find((p) => p.sourceBranch === branch.name);
	if (pr) return { text: `PR #${pr.number}`, colour: 'var(--lane-5)' };
	// The farm names a task's branch `spagitty-farm/<task>/<agent>`.
	const farm = /^spagitty-farm\/[^/]+\/([^/]+)/.exec(branch.name);
	if (farm) return { text: `farm · ${FARM_AGENTS[farm[1]] ?? farm[1]}`, colour: 'var(--accent)' };
	if (branch.name === defaultBranch) return { text: 'default', colour: 'var(--muted)' };
	if (/^release\//.test(branch.name)) return { text: 'release', colour: 'var(--muted)' };
	return null;
}

/** The two halves of the ahead/behind bar, as percentages of the widest drift. */
export function driftBar(branch: RecentBranch, widest: number): { ahead: number; behind: number } {
	const scale = Math.max(1, widest);
	const share = (n: number | null) => (n ? Math.max(8, Math.round((n / scale) * 100)) : 0);
	return { ahead: share(branch.ahead), behind: share(branch.behind) };
}

// ── Pull requests ────────────────────────────────────────────────────────

export interface PullRequestState {
	text: string;
	colour: string;
}

export function pullRequestState(pr: PullRequest, me: string | null): PullRequestState {
	if (pr.draft) return { text: 'Draft', colour: 'var(--muted)' };
	if (pr.reviewRequested && pr.authorName !== me) return { text: 'Your review', colour: 'var(--lane-5)' };
	if (pr.review === 'changesRequested') return { text: 'Changes requested', colour: 'var(--warn)' };
	if (pr.review === 'approved') return { text: 'Approved', colour: 'var(--ok)' };
	return { text: 'Open', colour: 'var(--muted)' };
}

/** Yours and those waiting on you first, then the rest, newest first within each. */
export function pullRequestOrder(prs: PullRequest[], me: string | null): PullRequest[] {
	const first = (pr: PullRequest) => (pr.needsYou || pr.reviewRequested || pr.authorName === me ? 0 : 1);
	return [...prs].sort((a, b) => first(a) - first(b) || b.updated - a.updated);
}

// ── Pipelines ────────────────────────────────────────────────────────────

export interface Workflow {
	name: string;
	/** Oldest first, at most fourteen: the newest is on the right. */
	runs: { id: number; state: 'ok' | 'failed' | 'running' | 'other'; height: number }[];
}

export const RUNS_SHOWN = 14;

export function workflows(runs: Run[]): Workflow[] {
	const byName = new Map<string, Run[]>();
	for (const run of runs) {
		const list = byName.get(run.name) ?? [];
		list.push(run);
		byName.set(run.name, list);
	}
	return [...byName.entries()].map(([name, list]) => ({
		name,
		runs: list
			.sort((a, b) => a.started - b.started)
			.slice(-RUNS_SHOWN)
			.map((run, i) => {
				const failed = ['failure', 'failed', 'timed_out', 'startup_failure'].includes(run.result);
				const state = run.state !== 'completed' ? 'running' : failed ? 'failed' : run.result === 'success' ? 'ok' : 'other';
				// Failed is the tallest, running next; passing runs vary a little so
				// a row of them reads as a history rather than a fence.
				const height = state === 'failed' ? 100 : state === 'running' ? 72 : 52 + ((i * 7) % 8) * 4;
				return { id: run.id, state, height };
			})
	}));
}
