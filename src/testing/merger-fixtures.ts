// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merger's test data (FEAT-100): the handoff's example, `main` and
 * `feat/tab-drag` split at `29c36a1`, four conflicts in three files.
 */

import type { BranchRow, MergerConflicts, MergerFile, MergerForecast, MergerSide } from '$lib/types';

export function side(overrides: Partial<MergerSide> = {}): MergerSide {
	return {
		name: 'main',
		kind: 'local',
		tip: 'a'.repeat(40),
		short: 'aaaaaaa',
		ahead: 4,
		newest: [
			{ id: 'e41c0b7'.padEnd(40, '0'), short: 'e41c0b7', summary: 'fix(chrome): hidden repositories leave the tab row', time: 1_700_000_000 },
			{ id: 'b7d2a19'.padEnd(40, '0'), short: 'b7d2a19', summary: 'feat(chrome): pinned tabs', time: 1_699_999_000 },
			{ id: 'a698930'.padEnd(40, '0'), short: 'a698930', summary: 'fix(forge): a GitLab token is proved against GitLab', time: 1_699_998_000 }
		],
		touching: 3,
		time: 1_700_000_000,
		checkedOut: '/repos/fixture',
		...overrides
	};
}

export function file(path: string, overrides: Partial<MergerFile> = {}): MergerFile {
	return {
		path,
		touch: 'both',
		addedByA: false,
		addedByB: false,
		deletedByA: false,
		deletedByB: false,
		conflicts: 0,
		kind: null,
		...overrides
	};
}

export function forecast(overrides: Partial<MergerForecast> = {}): MergerForecast {
	return {
		a: side(),
		b: side({
			name: 'feat/tab-drag',
			tip: 'b'.repeat(40),
			short: 'bbbbbbb',
			ahead: 5,
			touching: 3,
			checkedOut: null,
			newest: [
				{ id: '9e1b2c4'.padEnd(40, '0'), short: '9e1b2c4', summary: 'feat(chrome): tabs reorder by dragging', time: 1_700_000_100 }
			]
		}),
		base: '29c36a1'.padEnd(40, '0'),
		baseShort: '29c36a1',
		aHasB: false,
		bHasA: false,
		files: [
			file('src/lib/chrome/Tabs.svelte', { touch: 'conflict', conflicts: 2, kind: 'bothModified' }),
			file('src/lib/metrics.ts', { touch: 'conflict', conflicts: 1, kind: 'bothModified' }),
			file('CHANGELOG.md', { touch: 'conflict', conflicts: 1, kind: 'bothModified' }),
			file('crates/spagitty-core/src/repo.rs', { touch: 'both' }),
			file('src/lib/chrome/Tabs.test.ts', { touch: 'b', addedByB: true }),
			file('src/lib/nav.ts', { touch: 'b' }),
			file('crates/spagitty-core/src/forge/gitlab.rs', { touch: 'a' })
		],
		conflicts: 4,
		method: 'mergeTree',
		...overrides
	};
}

/** The same pair with nothing in conflict. */
export function cleanForecast(overrides: Partial<MergerForecast> = {}): MergerForecast {
	return forecast({
		conflicts: 0,
		files: [
			file('crates/spagitty-core/src/repo.rs', { touch: 'both' }),
			file('src/lib/nav.ts', { touch: 'b' }),
			file('crates/spagitty-core/src/forge/gitlab.rs', { touch: 'a' })
		],
		...overrides
	});
}

const TABS_A = `<script lang="ts">
	import { repos } from '$lib/repos';
	let tabs = $derived(repos.open.filter((r) => !r.hidden));
</script>

<div role="tablist" aria-label="Open repositories">
	{#each tabs as tab (tab.id)}
		<Tab {tab} pinned={tab.pinned}
			onclose={() => repos.close(tab.id)} />
	{/each}
</div>
`;

const TABS_B = `<script lang="ts">
	import { repos } from '$lib/repos';
	let tabs = $state([...repos.open]);
	let dragging = $state<string | null>(null);
</script>

<div role="tablist" aria-label="Open repositories">
	{#each tabs as tab (tab.id)}
		<Tab {tab} draggable="true"
			ondragstart={() => (dragging = tab.id)}
			onclose={() => repos.close(tab.id)} />
	{/each}
</div>
`;

const TABS_MERGED = `<script lang="ts">
	import { repos } from '$lib/repos';
<<<<<<< main
	let tabs = $derived(repos.open.filter((r) => !r.hidden));
||||||| 29c36a1
	let tabs = $derived(repos.open);
=======
	let tabs = $state([...repos.open]);
	let dragging = $state<string | null>(null);
>>>>>>> feat/tab-drag
</script>

<div role="tablist" aria-label="Open repositories">
	{#each tabs as tab (tab.id)}
<<<<<<< main
		<Tab {tab} pinned={tab.pinned}
			onclose={() => repos.close(tab.id)} />
||||||| 29c36a1
		<Tab {tab} onclose={() => repos.close(tab.id)} />
=======
		<Tab {tab} draggable="true"
			ondragstart={() => (dragging = tab.id)}
			onclose={() => repos.close(tab.id)} />
>>>>>>> feat/tab-drag
	{/each}
</div>
`;

const METRICS_A = 'export const metrics = {\n\trailWidth: 62,\n\ttabHeight: 30,\n\ttabGap: 4,\n\tpaneRadius: 18,\n};\n';
const METRICS_B = 'export const metrics = {\n\trailWidth: 62,\n\ttabHeight: 28,\n\ttabDragThreshold: 6,\n\tpaneRadius: 18,\n};\n';
const METRICS_MERGED =
	'export const metrics = {\n\trailWidth: 62,\n<<<<<<< main\n\ttabHeight: 30,\n\ttabGap: 4,\n||||||| 29c36a1\n\ttabHeight: 28,\n=======\n\ttabHeight: 28,\n\ttabDragThreshold: 6,\n>>>>>>> feat/tab-drag\n\tpaneRadius: 18,\n};\n';

const LOG_A = '## Unreleased\n\n- Hidden repositories leave the tab row, and tabs can be pinned.\n\n### Fixed\n';
const LOG_B = '## Unreleased\n\n- Tabs reorder by dragging, and the order survives a restart.\n\n### Fixed\n';
const LOG_MERGED =
	'## Unreleased\n\n<<<<<<< main\n- Hidden repositories leave the tab row, and tabs can be pinned.\n||||||| 29c36a1\n=======\n- Tabs reorder by dragging, and the order survives a restart.\n>>>>>>> feat/tab-drag\n\n### Fixed\n';

function text(value: string) {
	return { text: value, lines: value.split('\n').length - 1, bytes: value.length, binary: false, tooLarge: false };
}

function commit(short: string, summary: string) {
	return { id: short.padEnd(40, '0'), short, summary, time: 0 };
}

/** The handoff's four conflicts in three files, as the backend reads them. */
export function conflicts(): MergerConflicts {
	return {
		base: '29c36a1'.padEnd(40, '0'),
		baseShort: '29c36a1',
		aTip: 'a'.repeat(40),
		bTip: 'b'.repeat(40),
		files: [
			{
				path: 'src/lib/chrome/Tabs.svelte',
				kind: 'bothModified',
				base: null,
				a: text(TABS_A),
				b: text(TABS_B),
				merged: text(TABS_MERGED),
				regions: [
					{ index: 0, aLine: 3, bLine: 3, aCommit: commit('e41c0b7', 'fix(chrome): hidden repositories leave the tab row'), bCommit: commit('9e1b2c4', 'feat(chrome): tabs reorder by dragging') },
					{ index: 1, aLine: 8, bLine: 9, aCommit: commit('b7d2a19', 'feat(chrome): pinned tabs'), bCommit: commit('51d0a7e', 'feat(chrome): a dragged tab shows where it lands') }
				]
			},
			{
				path: 'src/lib/metrics.ts',
				kind: 'bothModified',
				base: null,
				a: text(METRICS_A),
				b: text(METRICS_B),
				merged: text(METRICS_MERGED),
				regions: [{ index: 0, aLine: 3, bLine: 3, aCommit: commit('b7d2a19', 'feat(chrome): pinned tabs'), bCommit: commit('51d0a7e', 'feat(chrome): a dragged tab shows where it lands') }]
			},
			{
				path: 'CHANGELOG.md',
				kind: 'bothModified',
				base: null,
				a: text(LOG_A),
				b: text(LOG_B),
				merged: text(LOG_MERGED),
				regions: [{ index: 0, aLine: 3, bLine: 3, aCommit: commit('5c3e8f0', 'chore(changelog): pinned tabs, BUG-043'), bCommit: commit('2f90e1d', 'chore(changelog): tab reorder') }]
			}
		]
	};
}

export function branchRow(name: string, overrides: Partial<BranchRow> = {}): BranchRow {
	return {
		name,
		fullName: `refs/heads/${name}`,
		kind: 'branch',
		current: false,
		id: 'c'.repeat(40),
		short: 'ccccccc',
		summary: 'A commit',
		authorName: 'Ada Lovelace',
		time: 1_700_000_000,
		upstream: null,
		ahead: null,
		behind: null,
		merged: false,
		...overrides
	};
}
