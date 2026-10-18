// SPDX-License-Identifier: GPL-3.0-or-later

import * as api from '$lib/api';
import * as issuesApi from '$lib/issues/api';
import type { Hosted, Issue } from '$lib/issues/types';
import { watcher } from '$lib/notifications/watcher.svelte';
import * as pipelinesApi from '$lib/pipelines/api';
import type { Run } from '$lib/pipelines/types';
import * as releasesApi from '$lib/releases/api';
import type { Release } from '$lib/releases/types';
import { requests } from '$lib/requests/store.svelte';
import { describe } from '$lib/ui/notice.svelte';
import type { StatusEntry } from '$lib/types';
import type { Overview } from './types';

/**
 * What Home shows (FEAT-140), read for the open repository.
 *
 * Two halves. The git half — the overview and the working copy — is read when
 * the screen opens and whenever the repository changes. The forge half — pull
 * requests, pipeline runs, releases and assigned issues — rides the watcher's
 * clock (FEAT-114) while Home is open; Home adds no timer of its own. Each part
 * has its own error, so a host that is down costs its cards and nothing else.
 */

let path = $state<string | null>(null);
let overview = $state<Overview | null>(null);
let changed = $state<StatusEntry[]>([]);
let host = $state<Hosted | null>(null);
let defaultRuns = $state<Run[]>([]);
let failedRuns = $state<Run[]>([]);
let latest = $state<Release | null>(null);
let assigned = $state<Issue[]>([]);
let loading = $state(false);
let errors = $state({ overview: '', pipelines: '', releases: '', issues: '' });
let epoch = 0;
let unsubscribe: (() => void) | null = null;

const FAILED = ['failure', 'failed', 'timed_out', 'startup_failure'];

/** The newest finished run on `branch` failed. */
function failing(runs: Run[], branch: string): Run | null {
	const done = runs
		.filter((run) => run.branch === branch && run.state === 'completed' && run.result && !['cancelled', 'canceled', 'skipped'].includes(run.result))
		.sort((a, b) => (b.ended || b.started) - (a.ended || a.started));
	return done[0] && FAILED.includes(done[0].result) ? done[0] : null;
}

async function readGit(ticket: number): Promise<void> {
	try {
		const [found, copy] = await Promise.all([api.homeOverview(), api.workingCopy().catch(() => null)]);
		if (ticket !== epoch) return;
		overview = found;
		errors.overview = '';
		if (copy) {
			// A path staged and changed again is one changed path.
			const seen = new Map<string, StatusEntry>();
			for (const entry of [...copy.conflicted, ...copy.staged, ...copy.unstaged]) {
				if (!seen.has(entry.path)) seen.set(entry.path, entry);
			}
			changed = [...seen.values()];
		}
	} catch (cause) {
		if (ticket === epoch) errors.overview = describe(cause);
	}
}

async function readForge(ticket: number, branch: string | null): Promise<void> {
	const reads: Promise<unknown>[] = [requests.load()];
	try {
		const hosts = await issuesApi.remotes();
		if (ticket !== epoch) return;
		host = hosts.find((h) => h.remote === 'origin') ?? hosts[0] ?? null;
	} catch {
		host = null;
	}
	const remote = host?.remote;
	if (remote) {
		const base = overview?.defaultBranch ?? branch;
		reads.push(
			(async () => {
				try {
					// The default branch for the card, and the person's own branches
					// for Needs you: the checked-out one and those of their pull
					// requests, at most three.
					const mine = new Set<string>();
					if (branch) mine.add(branch);
					for (const pr of requests.all) {
						if (requests.currentUser && pr.authorName === requests.currentUser) mine.add(pr.sourceBranch);
						if (mine.size >= 3) break;
					}
					const wanted = [...new Set([base, ...mine].filter((b): b is string => !!b))];
					const pages = await Promise.all(wanted.map((b) => pipelinesApi.runs(remote, b).then((r) => [b, r.data.runs] as const)));
					if (ticket !== epoch) return;
					defaultRuns = pages.find(([b]) => b === base)?.[1] ?? [];
					failedRuns = pages
						.filter(([b]) => mine.has(b))
						.map(([b, runs]) => failing(runs, b))
						.filter((run): run is Run => run !== null);
					errors.pipelines = '';
				} catch (cause) {
					if (ticket === epoch) errors.pipelines = describe(cause);
				}
			})(),
			(async () => {
				try {
					const page = await releasesApi.page(remote, 1);
					if (ticket !== epoch) return;
					latest = page.data.releases.find((r) => r.latest) ?? page.data.releases.find((r) => !r.draft && !r.prerelease) ?? null;
					errors.releases = '';
				} catch (cause) {
					if (ticket === epoch) errors.releases = describe(cause);
				}
			})(),
			(async () => {
				try {
					const page = await issuesApi.page(remote, 'is:open assignee:@me', 1);
					if (ticket !== epoch) return;
					assigned = page.data.issues;
					errors.issues = '';
				} catch (cause) {
					if (ticket === epoch) errors.issues = describe(cause);
				}
			})()
		);
	}
	await Promise.allSettled(reads);
}

export const home = {
	get path() {
		return path;
	},
	get overview() {
		return overview;
	},
	get changed() {
		return changed;
	},
	get host() {
		return host;
	},
	get defaultRuns() {
		return defaultRuns;
	},
	get failedRuns() {
		return failedRuns;
	},
	get latest() {
		return latest;
	},
	get assigned() {
		return assigned;
	},
	get loading() {
		return loading;
	},
	get errors() {
		return errors;
	},

	/** Read everything for the repository at `next`, and follow the forge's clock. */
	async open(next: string | null, branch: string | null): Promise<void> {
		const same = next === path;
		const ticket = ++epoch;
		path = next;
		if (!same) {
			overview = null;
			changed = [];
			host = null;
			defaultRuns = [];
			failedRuns = [];
			latest = null;
			assigned = [];
			errors = { overview: '', pipelines: '', releases: '', issues: '' };
		}
		if (!next || !api.inTauri()) return;
		loading = true;
		try {
			await readGit(ticket);
			await readForge(ticket, branch);
		} finally {
			if (ticket === epoch) loading = false;
		}
	},

	/** The git half again: the repository changed under the screen. */
	async refreshGit(): Promise<void> {
		if (path) await readGit(epoch);
	},

	/** While Home is on screen, its forge cards follow the watcher's clock. */
	follow(branch: () => string | null): () => void {
		unsubscribe?.();
		unsubscribe = watcher.subscribe(() => readForge(epoch, branch()));
		return () => {
			unsubscribe?.();
			unsubscribe = null;
		};
	},

	/** For tests. */
	present(next: {
		overview?: Overview | null;
		changed?: StatusEntry[];
		defaultRuns?: Run[];
		failedRuns?: Run[];
		latest?: Release | null;
		assigned?: Issue[];
		host?: Hosted | null;
	}): void {
		if (next.overview !== undefined) overview = next.overview;
		if (next.changed) changed = next.changed;
		if (next.defaultRuns) defaultRuns = next.defaultRuns;
		if (next.failedRuns) failedRuns = next.failedRuns;
		if (next.latest !== undefined) latest = next.latest;
		if (next.assigned) assigned = next.assigned;
		if (next.host !== undefined) host = next.host;
	}
};
