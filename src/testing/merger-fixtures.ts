// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merger's test data (FEAT-100): the handoff's example, `main` and
 * `feat/tab-drag` split at `29c36a1`, four conflicts in three files.
 */

import type { BranchRow, MergerFile, MergerForecast, MergerSide } from '$lib/types';

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
