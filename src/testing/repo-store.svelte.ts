// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A reactive stand-in for the repository store, for component tests.
 *
 * Reactive for the same reason as the graph stub: the chrome re-renders through
 * Svelte's reactivity, and a stub built from plain properties would render the
 * initial value and then never change.
 *
 * Lives outside `src/lib` so it is not counted as first-party code by the
 * coverage floor.
 */

import type { RepoCounts, RepoInfo } from '$lib/types';

const NO_COUNTS: RepoCounts = {
	commits: null,
	working: null,
	staged: null,
	conflicts: null,
	branches: null,
	stashes: null,
	tags: null,
	submodules: null
};

let info = $state<RepoInfo | null>(null);
let counts = $state<RepoCounts>(NO_COUNTS);
let error = $state<string | null>(null);
let busy = $state(false);
let generation = $state(0);
let token = $state<number | null>(null);

/** Calls the components made, for assertions. */
export const calls = {
	chosen: 0,
	opened: [] as string[],
	refreshed: 0,
	closed: 0
};

/** Makes the next `open` report failure, for the paths that handle one. */
let openFails = false;

export const control = {
	failNextOpen() {
		openFails = true;
	},
	setInfo(next: RepoInfo | null) {
		info = next;
	},
	setCounts(next: RepoCounts) {
		counts = next;
	},
	setError(next: string | null) {
		error = next;
	},
	setBusy(next: boolean) {
		busy = next;
	},
	setToken(next: number | null) {
		token = next;
	},
	reset() {
		openFails = false;
		info = null;
		counts = NO_COUNTS;
		error = null;
		busy = false;
		generation = 0;
		token = null;
		calls.chosen = 0;
		calls.opened = [];
		calls.refreshed = 0;
		calls.closed = 0;
	}
};

export const repo = {
	get info() {
		return info;
	},
	get counts() {
		return counts;
	},
	get error() {
		return error;
	},
	get busy() {
		return busy;
	},
	get generation() {
		return generation;
	},
	get token() {
		return token;
	},
	setToken(next: number) {
		token = next;
	},
	async open(path: string) {
		calls.opened.push(path);
		if (openFails) {
			openFails = false;
			return false;
		}
		generation += 1;
		return true;
	},
	async choose() {
		calls.chosen += 1;
		return true;
	},
	async refresh() {
		calls.refreshed += 1;
	},
	setCommitCount(n: number) {
		counts = { ...counts, commits: n };
	},
	/**
	 * Close, the way the real store closes.
	 *
	 * It used to call `control.reset()`, which clears the call counters as well
	 * as the state — so `close()` incremented `calls.closed` and then wiped it,
	 * and no test could ever observe that a close had happened. That is what
	 * hid BUG-019: the one assertion that would have caught the missing call
	 * was unwritable against this double.
	 *
	 * Only the state is cleared here. Resetting the counters belongs to
	 * `control.reset()`, which the suite calls between tests.
	 */
	async close() {
		calls.closed += 1;
		info = null;
		token = null;
		counts = NO_COUNTS;
		generation += 1;
	}
};
