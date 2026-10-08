// SPDX-License-Identifier: GPL-3.0-or-later

import type { Settings, Watched } from '$lib/types';

/**
 * What changed between two reads of the watched pull requests (FEAT-114).
 *
 * Pure: the watcher keeps the last answer and hands both here. The first read
 * has nothing to compare with and announces nothing — turning notifications on
 * must not open with a burst of everything that ever happened.
 */

export type NewsKind = 'merged' | 'closed' | 'comment' | 'reviewRequested' | 'checksFailed';

export interface News {
	kind: NewsKind;
	pr: Watched;
	/** The headline: what happened. */
	title: string;
	/** Which pull request it happened to. */
	body: string;
}

/** The settings that choose which kinds are worth saying. */
export type Wants = Pick<
	Settings,
	'notifyMerged' | 'notifyComments' | 'notifyReviewRequests' | 'notifyChecks'
>;

const HEADLINE: Record<NewsKind, (pr: Watched) => string> = {
	merged: () => 'Your pull request was merged',
	closed: () => 'Your pull request was closed',
	comment: (pr) => (pr.lastActor ? `${pr.lastActor} commented` : 'New comments'),
	reviewRequested: () => 'Review requested',
	checksFailed: () => 'Checks failed'
};

function news(kind: NewsKind, pr: Watched): News {
	return {
		kind,
		pr,
		title: HEADLINE[kind](pr),
		body: `${pr.title} · ${pr.repository}#${pr.number}`
	};
}

/** What `now` says that `before` did not, in the order `now` lists it. */
export function changes(
	before: Record<string, Watched> | null,
	now: Watched[],
	wants: Wants
): News[] {
	if (!before) return [];

	const found: News[] = [];
	for (const pr of now) {
		const was = before[pr.key];

		if (!was) {
			// A pull request seen for the first time is news only when it asks
			// something of the person. Their own new one is not news to them.
			if (wants.notifyReviewRequests && pr.reviewRequested && pr.state === 'open') {
				found.push(news('reviewRequested', pr));
			}
			continue;
		}

		if (pr.mine && was.state === 'open' && pr.state !== 'open') {
			if (wants.notifyMerged) found.push(news(pr.state === 'merged' ? 'merged' : 'closed', pr));
			continue;
		}
		if (
			wants.notifyReviewRequests &&
			pr.reviewRequested &&
			!was.reviewRequested &&
			pr.state === 'open'
		) {
			found.push(news('reviewRequested', pr));
		}
		if (wants.notifyComments && pr.activity > was.activity && !pr.lastActorIsMe) {
			found.push(news('comment', pr));
		}
		if (
			wants.notifyChecks &&
			pr.mine &&
			pr.state === 'open' &&
			pr.checks === 'failing' &&
			was.checks !== 'failing'
		) {
			found.push(news('checksFailed', pr));
		}
	}
	return found;
}
