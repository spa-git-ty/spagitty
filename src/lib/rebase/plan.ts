// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What the Rebase screen says about a plan (FEAT-106), kept out of the markup
 * so the words and the counts are tested rather than read off a screenshot.
 *
 * The screen is drawn after Merger: the branch being moved on one side, the
 * branch it lands on on the other, and the result between them, before
 * anything is written.
 */

import type { RebaseAction, RebaseEdit, RebasePreview } from '../types';

export interface PlanCounts {
	/** Commits the branch will have after the rebase. */
	after: number;
	/** Commits that move — every one not dropped, folded ones included. */
	replayed: number;
	squashed: number;
	reworded: number;
	dropped: number;
	/** Rows the preview thinks may not apply cleanly. */
	risky: number;
}

export function counts(plan: RebaseEdit[], preview: RebasePreview | null): PlanCounts {
	const of = (action: RebaseAction) => plan.filter((entry) => entry.action === action).length;
	const dropped = of('drop');
	return {
		after: preview ? preview.rows.length : plan.length - dropped - of('squash'),
		replayed: plan.length - dropped,
		squashed: of('squash'),
		reworded: of('reword'),
		dropped,
		risky: preview ? preview.rows.filter((row) => row.mayConflict).length : 0
	};
}

const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** One sentence for the Result card: what happens to the branch. */
export function sentence(branch: string, onto: string, c: PlanCounts): string {
	if (c.after === 0) {
		return `Every commit is dropped: ${branch} ends up exactly at ${onto}.`;
	}
	const parts = [`${plural(c.replayed, 'commit')} of ${branch} are replayed on top of ${onto}`];
	const changes: string[] = [];
	if (c.squashed > 0) changes.push(`${c.squashed} folded into the one above`);
	if (c.reworded > 0) changes.push(`${c.reworded} with a new message`);
	if (c.dropped > 0) changes.push(`${c.dropped} left out`);
	const tail = changes.length > 0 ? `, ${changes.join(', ')}` : '';
	return `${parts[0]}${tail}. ${branch} moves to the last of them, with new ids.`;
}

/** The plan's actions, as the screen offers them: a word, an icon, a colour. */
export const ACTION_LOOK: Record<RebaseAction, { label: string; tone: string; hint: string }> = {
	pick: { label: 'Pick', tone: 'pick', hint: 'Keep this commit as it is' },
	reword: { label: 'Reword', tone: 'reword', hint: 'Keep the change, give it a new message' },
	squash: { label: 'Squash', tone: 'squash', hint: 'Fold this commit into the one above it' },
	drop: { label: 'Drop', tone: 'drop', hint: 'Leave this commit out of the result' }
};

/** A squash with nothing above it to fold into: git refuses that plan. */
export function orphanSquash(plan: RebaseEdit[], id: string): boolean {
	for (const entry of plan) {
		if (entry.action === 'drop') continue;
		return entry.id === id && entry.action === 'squash';
	}
	return false;
}
