// SPDX-License-Identifier: GPL-3.0-or-later

import { goto } from '$app/navigation';
import { farmStore } from '$lib/farm/store.svelte';
import { openInMerger } from '$lib/farm/merge';
import { fetchAll, pull } from '$lib/graph/actions';
import { requests } from '$lib/requests/store.svelte';
import { review } from '$lib/review/store.svelte';
import type { Target } from './center';

/**
 * Doing what a row's button says (FEAT-140, FEAT-142).
 *
 * Home's Needs you and the notification center offer the same button for a
 * kind, and this is the one place that knows what each does. Most go where the
 * row is about; three act there and then — retry the fetch or the pull, and
 * open a ready farm task in Merger, which is where every merge is made
 * (FEAT-117).
 */

/** Follow a target: a screen of the application, or a page in the browser. */
export function follow(target: Target): void {
	if (target.kind === 'route') {
		void goto(target.route);
	} else if (target.kind === 'url') {
		// The same way every host link in the application opens: an anchor the
		// shell sends to the browser, not a navigation of the window.
		const link = document.createElement('a');
		link.href = target.url;
		link.target = '_blank';
		link.rel = 'noreferrer';
		link.click();
	}
}

/** The farm task a target names, when it names one. */
export function taskOf(target: Target): string | null {
	if (target.kind !== 'route' || !target.route.startsWith('/farm?')) return null;
	return new URLSearchParams(target.route.slice('/farm?'.length)).get('task');
}

/** Run the action a kind's button names. */
export async function act(kind: string, target: Target): Promise<void> {
	switch (kind) {
		case 'fetchFailed':
			await fetchAll();
			return;
		case 'pullFailed':
		case 'pushFailed':
			await pull();
			return;
		case 'mergeReady': {
			const id = taskOf(target);
			const task = id ? farmStore.tasks.find((t) => t.id === id) : undefined;
			if (task && (await openInMerger(task))) return;
			follow(target);
			return;
		}
		case 'reviewRequested':
		case 'changesRequested':
		case 'comment':
		case 'checksFailed': {
			// A pull request of the open repository is read in Review, as Home
			// opens it; one elsewhere opens on its host.
			const url = target.kind === 'url' ? target.url : null;
			const number = url ? numberIn(url, requests.repo) : null;
			const pr = number === null ? undefined : requests.all.find((p) => p.number === number);
			if (pr && (await review.open(pr))) {
				void goto('/review');
				return;
			}
			follow(target);
			return;
		}
		default:
			follow(target);
	}
}

/**
 * The number of the pull request a host page is about, when the page is in the
 * open repository: `https://github.com/owner/name/pull/66` is 66 there.
 */
export function numberIn(url: string, repo: { host: string; owner: string; name: string } | null): number | null {
	if (!repo) return null;
	const here = `${repo.host}/${repo.owner}/${repo.name}/`.toLowerCase();
	if (!url.toLowerCase().includes(here)) return null;
	const found = /\/(?:pull|pulls|merge_requests|pull-requests)\/(\d+)/.exec(url);
	return found ? Number(found[1]) : null;
}
