// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Fetching and pushing, while they are happening (FEAT-018).
 *
 * Both were fire-and-wait: `--progress` was passed to git and nothing read it,
 * so a large fetch said nothing for a minute and then said everything. That is
 * indistinguishable from an application that has hung, and it is what this
 * store fixes — the work runs on a worker and every line git writes arrives as
 * an event.
 *
 * The listener belongs to the layout, not to a screen. A fetch survives
 * navigation, and someone who starts one from the toolbar and walks to the
 * Branches screen should not stop hearing about it.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from '../api';
import { repo } from '../repo.svelte';
import { pushed } from '$lib/delight/watch';
import { notice } from '../ui/notice.svelte';
import { settings } from '../settings/store.svelte';
import {
	NETWORK_DONE_EVENT,
	NETWORK_PROGRESS_EVENT,
	type CloneProgress,
	type NetworkDoneEvent,
	type NetworkOperation,
	type NetworkProgressEvent
} from '../types';

/** The operation running right now, by token. Null when none is. */
let token = $state<number | null>(null);
let operation = $state<NetworkOperation | null>(null);
/** The last step git reported, or null before the first one. */
let step = $state<CloneProgress | null>(null);
/** git's own last words about the operation that finished. */
let summary = $state<string | null>(null);
let error = $state<string | null>(null);
/** Whoever is waiting for the operation running now to end. */
let waiting: ((ok: boolean) => void)[] = [];

export const network = {
	get running(): boolean {
		return token !== null;
	},
	get operation(): NetworkOperation | null {
		return operation;
	},
	get step(): CloneProgress | null {
		return step;
	},
	get summary(): string | null {
		return summary;
	},
	get error(): string | null {
		return error;
	},

	/**
	 * What to show while it runs.
	 *
	 * git's own words, because they are better than anything invented in their
	 * place — and a phase with no percentage is still worth showing, since
	 * "Enumerating objects" tells you it is alive.
	 */
	get label(): string {
		if (token === null) return '';
		if (step === null) return operation === 'push' ? 'Pushing…' : 'Fetching…';
		return step.percent === null ? step.line : `${step.phase} ${step.percent}%`;
	},

	/**
	 * Fetch. Resolves when it has *started*.
	 *
	 * Pruning is read from the settings here rather than passed in by each
	 * caller, so there is one place that decides it and no button that can
	 * quietly prune when the setting says not to.
	 */
	async fetch(remote = ''): Promise<boolean> {
		return this.start(() => api.fetch(remote, settings.settings.pruneOnFetch), 'fetch');
	},

	/** Push. Resolves when it has started. */
	async push(remote = '', refspec = ''): Promise<boolean> {
		return this.start(() => api.push(remote, refspec, false), 'push');
	},

	async start(begin: () => Promise<number>, next: NetworkOperation): Promise<boolean> {
		if (token !== null) return false;

		operation = next;
		step = null;
		summary = null;
		error = null;

		try {
			token = await begin();
			return true;
		} catch (e) {
			token = null;
			operation = null;
			error = String(e);
			return false;
		}
	},

	/**
	 * How the operation running now ends: true when it went well, and `error`
	 * holds git's words when it did not. `fetch` and `push` resolve when the
	 * work has started; a caller whose next step depends on the outcome waits
	 * here (BUG-076).
	 */
	whenDone(): Promise<boolean> {
		if (token === null) return Promise.resolve(error === null);
		return new Promise((resolve) => waiting.push(resolve));
	},

	/** Listen for progress. Attached by the layout before anything can emit. */
	async attach(): Promise<() => void> {
		const offProgress: UnlistenFn = await listen<NetworkProgressEvent>(
			NETWORK_PROGRESS_EVENT,
			(event) => {
				if (event.payload.token !== token) return;
				step = {
					phase: event.payload.phase,
					percent: event.payload.percent,
					line: event.payload.line
				};
			}
		);

		const offDone: UnlistenFn = await listen<NetworkDoneEvent>(NETWORK_DONE_EVENT, (event) => {
			if (event.payload.token !== token) return;
			token = null;
			step = null;
			// The worker cannot let go of itself — dropping it joins its own
			// thread — so the screen says when it is finished with.
			void api.networkRelease();

			const what = event.payload.operation === 'push' ? 'Pushed' : 'Fetched';

			if (event.payload.ok) {
				summary = event.payload.summary;
				error = null;
				// The notice as well as the toolbar line: the toolbar's is
				// transient state that the next operation overwrites, and a
				// notice is what somebody looking at another screen sees.
				notice.ok(what, summary);
				// Spagitty never passes `--force-with-lease` from the UI today,
				// so this is always the gentle kind. The flag is carried anyway
				// so that the day it can, the badge is already wired (FEAT-072).
				if (event.payload.operation === 'push') pushed(false);
				// A fetch moves remote-tracking refs and a push moves the
				// upstream; both change what every divergence on screen means.
				void repo.refresh();
			} else {
				error = event.payload.error;
				summary = null;
				notice.failed(`Could not ${event.payload.operation}`, error);
			}
			for (const resolve of waiting.splice(0)) resolve(event.payload.ok);
		});

		return () => {
			offProgress();
			offDone();
		};
	},

	clear(): void {
		// Not `token`: an operation that is running belongs to the repository
		// rather than to a screen, and forgetting it would orphan its events.
		step = null;
		summary = null;
		error = null;
	}
};
