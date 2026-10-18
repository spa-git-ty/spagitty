// SPDX-License-Identifier: GPL-3.0-or-later

import { listen } from '@tauri-apps/api/event';
import { EVENT as FARM_EVENT, farmStore } from '$lib/farm/store.svelte';
import type { FarmStatus, RecordedEvent } from '$lib/farm/types';
import * as pipelines from '$lib/pipelines/api';
import { repo } from '$lib/repo.svelte';
import { settings } from '$lib/settings/store.svelte';
import { notice } from '$lib/ui/notice.svelte';
import { center } from './center.svelte';
import { fromFarmEvent, fromNotice, fromRuns, type Outcome } from './feeds';
import { watcher } from './watcher.svelte';

/**
 * What feeds the notification center (FEAT-142), attached once by the layout.
 *
 * The pull request watcher writes its own news (it knows when it announces);
 * everything else is heard here: the notices that say a fetch, pull or push
 * failed, the farm's events, and the checks on the branch that is checked out.
 * None of it changes what is said in the corner; it is only kept.
 */

/** The farm's last status, by repository, so only a change is news. */
const farmStatus = new Map<string, FarmStatus>();

/** The last outcome seen per `remote:branch`, kept so a restart compares. */
const SEEN_RUNS = 'spagitty.notifications.runs';

function seenRuns(): Record<string, Outcome | null> {
	try {
		return JSON.parse(localStorage.getItem(SEEN_RUNS) ?? '{}') as Record<string, Outcome | null>;
	} catch {
		return {};
	}
}

function rememberRun(key: string, outcome: Outcome | null): void {
	try {
		localStorage.setItem(SEEN_RUNS, JSON.stringify({ ...seenRuns(), [key]: outcome }));
	} catch {
		// Storage refused: the next look compares with nothing and stays quiet.
	}
}

/** Look at the checked-out branch's runs once, and keep a change. */
export async function checkRuns(): Promise<void> {
	const info = repo.info;
	const branch = info?.head.branch;
	if (!info || !branch) return;
	const hosted = await pipelines.remotes().catch(() => []);
	const remote = hosted[0]?.remote;
	if (!remote) return;
	const read = await pipelines.runs(remote, branch).catch(() => null);
	if (!read || repo.info?.path !== info.path) return;
	const key = `${info.path}:${remote}:${branch}`;
	const seen = seenRuns();
	const { now, said } = fromRuns(branch, read.data.runs, key in seen ? seen[key] : undefined);
	rememberRun(key, now);
	if (said) void center.add({ ...said, repository: info.path, repositoryName: info.name });
}

export async function attach(): Promise<() => void> {
	void center.load();

	const offNotice = notice.listen((said) => {
		const kept = fromNotice(said);
		if (kept) void center.add(kept);
	});

	const offFarm = await listen<RecordedEvent>(FARM_EVENT, (event) => {
		const path = farmStore.path;
		if (!path) return;
		const previous = farmStatus.get(path) ?? farmStore.farm?.status ?? null;
		if (event.payload.kind === 'farmStatusChanged') farmStatus.set(path, event.payload.status);
		const kept = fromFarmEvent(event.payload, farmStore.farm, previous);
		if (!kept) return;
		const name = path.split(/[\\/]/).filter(Boolean).pop() ?? path;
		void center.add({ ...kept, repository: path, repositoryName: name });
	});

	// The checks ride the watcher's clock (FEAT-114), at its interval, and only
	// while the person has asked to hear about checks at all.
	let offRuns: (() => void) | null = null;
	const every = () => Math.max(1, settings.settings.notifyEveryMinutes) * 60_000;
	const wanted = () => settings.settings.notifyPullRequests && settings.settings.notifyChecks;
	const offWatch = $effect.root(() => {
		$effect(() => {
			const on = wanted();
			const period = every();
			offRuns?.();
			offRuns = on ? watcher.subscribe(checkRuns, period) : null;
		});
	});

	return () => {
		offNotice();
		offFarm();
		offRuns?.();
		offWatch();
	};
}
