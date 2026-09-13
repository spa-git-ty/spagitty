// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The open repository: which one, what HEAD is, and the rail counts.
 *
 * One repository is open at a time. Opening another replaces it, which on the
 * Rust side tears down the previous walk and watcher.
 */

import { open as openDialog } from '@tauri-apps/plugin-dialog';
import * as api from './api';
import { graphOrder } from './graph/order.svelte';
import { workspace } from './workspace.svelte';
import type { RepoCounts, RepoInfo } from './types';

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

/** Bumped on every successful open, so the graph knows to start over. */
let generation = $state(0);
let token = $state<number | null>(null);

export const repo = {
	get info(): RepoInfo | null {
		return info;
	},
	get counts(): RepoCounts {
		return counts;
	},
	get error(): string | null {
		return error;
	},
	get busy(): boolean {
		return busy;
	},
	get generation(): number {
		return generation;
	},
	/** The graph walk token for the current repository. */
	get token(): number | null {
		return token;
	},

	setToken(next: number) {
		token = next;
	},

	async open(path: string): Promise<boolean> {
		busy = true;
		error = null;
		try {
			const result = await api.openRepo(path, graphOrder.id);
			info = result.info;
			counts = result.counts;
			token = result.token;
			generation += 1;
			// Every way into a repository lands here — the tab strip, the All
			// repositories screen, the launch argument, a finished clone — so
			// this is the one place that has to record it as open.
			workspace.opened(result.info.path);
			return true;
		} catch (e) {
			error = String(e);
			info = null;
			token = null;
			counts = NO_COUNTS;
			return false;
		} finally {
			busy = false;
		}
	},

	/** Ask the user for a directory, then open it. */
	async choose(): Promise<boolean> {
		const picked = await openDialog({ directory: true, multiple: false, title: 'Open repository' });
		if (typeof picked !== 'string') return false;
		return this.open(picked);
	},

	/** Re-read HEAD and the counts. Called after the watcher reports a change. */
	async refresh(): Promise<void> {
		if (!info) return;
		try {
			const snap = await api.snapshot();
			info = snap.info;
			counts = { ...snap.counts, commits: counts.commits };
			error = null;
		} catch (e) {
			error = String(e);
		}
	},

	/** The graph reports its own row count, which is the only real commit count. */
	setCommitCount(n: number) {
		counts = { ...counts, commits: n };
	},

	/**
	 * Close the repository. Nothing is open afterwards.
	 *
	 * The generation moves, exactly as it does on `open` (BUG-019). It is the
	 * signal every screen keys off to notice the repository underneath it has
	 * changed — the shell restarts the walk on it, and the screens that cache
	 * per repository re-sync on it. Without the bump the state cleared here and
	 * the state held out there disagreed: no repository, and a graph still full
	 * of its commits.
	 *
	 * `graph.restart()` then resets to nothing and returns, because there is no
	 * token to walk with, which is exactly what closing should leave behind.
	 */
	async close(): Promise<void> {
		await api.closeRepo();
		info = null;
		token = null;
		counts = NO_COUNTS;
		generation += 1;
	}
};
