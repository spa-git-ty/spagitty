// SPDX-License-Identifier: GPL-3.0-or-later

import type { IconName } from '$lib/ui/icons';

/**
 * Every kind of thing Spagitty tells the person, and how each one looks and
 * what its button does (FEAT-140, FEAT-142).
 *
 * One table, read by Home's Needs you and by the notification center, so a
 * review request is the same blue with the same eye and the same `Review`
 * button wherever it appears. Two tables would drift, and a person who learns
 * the colour of "stuck" on one screen should not have to learn it again.
 */

/** Which chip of the drawer a kind is under. `issues` is on Home only. */
export type Category = 'pullRequests' | 'farm' | 'checks' | 'git' | 'issues';

export type Kind =
	// Pull requests: the watcher's five (FEAT-114), and a review asking for changes.
	| 'merged'
	| 'closed'
	| 'comment'
	| 'reviewRequested'
	| 'checksFailed'
	| 'changesRequested'
	// The farm.
	| 'mergeReady'
	| 'taskStuck'
	| 'farmStopped'
	| 'farmFinished'
	// A branch's own pipeline runs.
	| 'runFailed'
	| 'runPassing'
	// Git.
	| 'fetchFailed'
	| 'pullFailed'
	| 'pushFailed'
	| 'releasePublished'
	// Issues.
	| 'assigned';

export interface KindStyle {
	category: Category;
	icon: IconName;
	/** A palette token, as `var(...)`. The tile is this colour at a low mix. */
	colour: string;
	/** Home's word for it, in its colour, before the title. */
	label: string;
	/** The one button. */
	action: string;
	/** Whether the button is the filled one. Only what moves work on is. */
	primary: boolean;
}

export const KINDS: Record<Kind, KindStyle> = {
	mergeReady: {
		category: 'farm',
		icon: 'merge',
		colour: 'var(--ok)',
		label: 'Merge ready',
		action: 'Approve merge',
		primary: true
	},
	reviewRequested: {
		category: 'pullRequests',
		icon: 'review',
		colour: 'var(--lane-5)',
		label: 'Review requested',
		action: 'Review',
		primary: true
	},
	taskStuck: {
		category: 'farm',
		icon: 'warning',
		colour: 'var(--danger)',
		label: 'Task stuck',
		action: 'Open task',
		primary: false
	},
	runFailed: {
		category: 'checks',
		icon: 'warning',
		colour: 'var(--danger)',
		label: 'Checks failed',
		action: 'See the run',
		primary: false
	},
	changesRequested: {
		category: 'pullRequests',
		icon: 'comment',
		colour: 'var(--warn)',
		label: 'Changes requested',
		action: 'Open',
		primary: false
	},
	assigned: {
		category: 'issues',
		icon: 'issues',
		colour: 'var(--lane-4)',
		label: 'Assigned to you',
		action: 'Open',
		primary: false
	},
	merged: {
		category: 'pullRequests',
		icon: 'merge',
		colour: 'var(--lane-4)',
		label: 'Merged',
		action: 'Open',
		primary: false
	},
	closed: {
		category: 'pullRequests',
		icon: 'merge',
		colour: 'var(--muted)',
		label: 'Closed',
		action: 'Open',
		primary: false
	},
	comment: {
		category: 'pullRequests',
		icon: 'comment',
		colour: 'var(--warn)',
		label: 'Comment',
		action: 'Open',
		primary: false
	},
	checksFailed: {
		category: 'pullRequests',
		icon: 'warning',
		colour: 'var(--danger)',
		label: 'Checks failed',
		action: 'Open',
		primary: false
	},
	farmStopped: {
		category: 'farm',
		icon: 'farm',
		colour: 'var(--warn)',
		label: 'Farm stopped',
		action: 'Open the farm',
		primary: false
	},
	farmFinished: {
		category: 'farm',
		icon: 'farm',
		colour: 'var(--ok)',
		label: 'Farm finished',
		action: 'Wrap up',
		primary: false
	},
	runPassing: {
		category: 'checks',
		icon: 'circle-check',
		colour: 'var(--ok)',
		label: 'Passing again',
		action: 'See the run',
		primary: false
	},
	fetchFailed: {
		category: 'git',
		icon: 'fetch',
		colour: 'var(--muted)',
		label: 'Fetch failed',
		action: 'Fetch again',
		primary: false
	},
	pullFailed: {
		category: 'git',
		icon: 'pull',
		colour: 'var(--muted)',
		label: 'Pull failed',
		action: 'Pull again',
		primary: false
	},
	pushFailed: {
		category: 'git',
		icon: 'push',
		colour: 'var(--muted)',
		label: 'Push failed',
		action: 'Pull',
		primary: false
	},
	releasePublished: {
		category: 'git',
		icon: 'releases',
		colour: 'var(--warn)',
		label: 'Released',
		action: 'Open',
		primary: false
	}
};

/** The style of a stored kind, or a quiet default for one a later build wrote. */
export function styleOf(kind: string): KindStyle {
	return (
		KINDS[kind as Kind] ?? {
			category: 'git',
			icon: 'circle',
			colour: 'var(--muted)',
			label: kind,
			action: 'Open',
			primary: false
		}
	);
}
