// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Review inbox, worked out from the pull request list (FEAT-087).
 *
 * Pure: a list and the saved records in, groups and words out. The screen
 * draws what this returns, and `inbox.test.ts` holds the rules — which group
 * a pull request lands in, how big it reads, what its progress says — without
 * mounting anything.
 */

import type { PullRequest } from '../types';
import { viewedCount, type ReviewRecord } from './record';

export type GroupId = 'needs' | 'back' | 'open' | 'mine';

export interface InboxGroup {
	id: GroupId;
	title: string;
	hint: string;
	items: PullRequest[];
}

/**
 * Four groups, by what each pull request needs from you.
 *
 * - **Needs you**: you are a requested reviewer.
 * - **Back with you**: not requested now, but a thread you started has an
 *   answer — the author replied and it is your move.
 * - **Open**: everything else somebody else opened.
 * - **Yours**: what you opened, last, so its threads can be read and answered
 *   here. Leaving them out told the author of every pull request on a
 *   repository that there was nothing to review (BUG-057).
 */
export function groupInbox(
	list: PullRequest[],
	me: string | null,
	scope: 'repo' | 'all' = 'repo'
): InboxGroup[] {
	const mine = (pr: PullRequest) => !!me && pr.authorName.toLowerCase() === me.toLowerCase();
	const others = list.filter((pr) => !mine(pr));

	const needs = others.filter((pr) => pr.reviewRequested);
	const back = others.filter((pr) => !pr.reviewRequested && pr.repliesToYou > 0);
	const open = others.filter((pr) => !pr.reviewRequested && pr.repliesToYou === 0);

	const groups: InboxGroup[] = [
		{ id: 'needs', title: 'Needs you', hint: 'you are a requested reviewer', items: needs },
		{ id: 'back', title: 'Back with you', hint: 'the author answered you', items: back },
		{
			id: 'open',
			title: scope === 'repo' ? 'Open on this repo' : 'Involving you',
			hint: 'nobody asked you yet',
			items: open
		},
		{ id: 'mine', title: 'Yours', hint: 'you opened these', items: list.filter(mine) }
	];
	return groups.filter((group) => group.items.length > 0);
}

/** Every pull request in the order the groups show them. */
export function inboxOrder(groups: InboxGroup[]): PullRequest[] {
	return groups.flatMap((group) => group.items);
}

/**
 * How big a pull request is, as one to three bars.
 *
 * By lines changed when the host says, and by files when it does not — GitLab
 * lists a merge request without its line counts.
 */
export function sizeOf(pr: PullRequest): 1 | 2 | 3 {
	const lines = pr.added + pr.removed;
	if (lines > 0) return lines <= 100 ? 1 : lines <= 500 ? 2 : 3;
	return pr.changedFiles <= 5 ? 1 : pr.changedFiles <= 20 ? 2 : 3;
}

/** `6 files · +16 −8`, or the files alone when the lines are not known. */
export function sizeLabel(pr: PullRequest): { files: string; lines: string | null } {
	const files = `${pr.changedFiles} ${pr.changedFiles === 1 ? 'file' : 'files'}`;
	const known = pr.added + pr.removed > 0;
	return {
		files,
		lines: known ? `+${pr.added.toLocaleString('en')} −${pr.removed.toLocaleString('en')}` : null
	};
}

export interface Progress {
	/** 0 to 1, for the bar. */
	share: number;
	label: string;
	/** True when the author pushed after the reviewer last looked. */
	changed: boolean;
}

/**
 * How far the reviewer got.
 *
 * Counted against the host's file count, since that is the number the card
 * shows. A head that moved since the last look is said, rather than a count
 * that may no longer be true: which ticks survive is only known once the new
 * version has been read.
 */
export function progressOf(pr: PullRequest, record: ReviewRecord | null): Progress {
	const total = Math.max(pr.changedFiles, record?.files ?? 0);
	const viewed = record ? Math.min(viewedCount(record), total) : 0;
	const share = total > 0 ? viewed / total : 0;

	if (record && viewed > 0 && pr.headSha && record.headSha && record.headSha !== pr.headSha) {
		return { share, label: 'changed since you looked', changed: true };
	}
	if (viewed === 0) return { share: 0, label: 'not started', changed: false };
	return { share, label: `${viewed} of ${total} viewed`, changed: false };
}

/** What the card's chips say, beside the branch. */
export interface Chips {
	/** The host cannot merge it into its base as it stands (BUG-063). */
	base: string | null;
	conflict: string | null;
	threads: string | null;
}

export function chipsOf(pr: PullRequest, record: ReviewRecord | null, group: GroupId): Chips {
	const base = pr.mergeable === false ? `conflicts with ${pr.targetBranch}` : null;
	const conflict = record && record.conflictFiles.length > 0 ? 'conflict fixes' : null;
	if (group === 'back') {
		const n = pr.repliesToYou;
		return { base, conflict, threads: `${n} ${n === 1 ? 'reply' : 'replies'} to you` };
	}
	const n = pr.openThreads;
	return { base, conflict, threads: n > 0 ? `${n} open ${n === 1 ? 'thread' : 'threads'}` : null };
}

export type FactTone = 'resolve' | 'accent' | 'ok' | 'danger' | 'warn' | 'muted';

export interface Fact {
	tone: FactTone;
	text: string;
}

/** The preview card's "Before you start": what is worth knowing first. */
export function factsOf(pr: PullRequest, record: ReviewRecord | null): Fact[] {
	const facts: Fact[] = [];

	if (pr.mergeable === false) {
		facts.push({
			tone: 'danger',
			text: `Conflicts with ${pr.targetBranch}: it cannot be merged as it stands`
		});
	}

	if (record && record.conflictFiles.length > 0) {
		const n = record.conflictFiles.length;
		const merges = record.conflictMerges.length > 0 ? ` from merge ${record.conflictMerges.join(', ')}` : '';
		facts.push({
			tone: 'resolve',
			text: `${n} ${n === 1 ? 'file carries' : 'files carry'} conflict fixes${merges}`
		});
	}

	if (pr.openThreads + pr.resolvedThreads > 0) {
		const open = `${pr.openThreads} open ${pr.openThreads === 1 ? 'thread' : 'threads'}`;
		facts.push({
			tone: 'accent',
			text: pr.resolvedThreads > 0 ? `${open}, ${pr.resolvedThreads} resolved` : open
		});
	}

	if (pr.checks === 'passing') facts.push({ tone: 'ok', text: 'Checks passing' });
	if (pr.checks === 'failing') facts.push({ tone: 'danger', text: 'Checks failing' });
	if (pr.checks === 'running') facts.push({ tone: 'muted', text: 'Checks running' });

	const drafts = record?.drafts.length ?? 0;
	if (drafts > 0) {
		facts.push({
			tone: 'warn',
			text: `${drafts} pending ${drafts === 1 ? 'comment' : 'comments'} of yours`
		});
	}

	return facts;
}

/** The primary button's words. */
export function continueLabel(progress: Progress): string {
	return progress.share > 0 ? `Continue review · ${progress.label}` : 'Start review';
}
