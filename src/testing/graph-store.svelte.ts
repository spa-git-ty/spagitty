// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A reactive stand-in for the graph store, for component tests.
 *
 * It has to hold its state in `$state` rather than in a plain object: the
 * components under test re-render through Svelte's reactivity, so a stub whose
 * fields are ordinary properties would render once and then never update — and
 * every test about a change would silently pass by rendering the initial value.
 *
 * Lives outside `src/lib` so it is not counted as first-party code under
 * Amendment 10.
 */

import { lanesNeeded } from '$lib/graph/lanes';
import type { CommitDetail, GraphRow } from '$lib/types';

let rows = $state<GraphRow[]>([]);
let selectedIndex = $state<number | null>(null);
let detail = $state<CommitDetail | null>(null);
let detailError = $state<string | null>(null);
let error = $state<string | null>(null);
let complete = $state(true);
let refreshedAt = $state<number | null>(null);
let version = $state(0);

/** Calls the components made, for assertions. */
export const calls = {
	ensured: [] as number[],
	selected: [] as number[]
};

/** Test-side control over what the stub reports. */
export const control = {
	setRows(next: GraphRow[]) {
		rows = next;
		version += 1;
	},
	setSelected(index: number | null) {
		selectedIndex = index;
	},
	setDetail(next: CommitDetail | null) {
		detail = next;
	},
	setDetailError(next: string | null) {
		detailError = next;
	},
	setError(next: string | null) {
		error = next;
	},
	setComplete(next: boolean) {
		complete = next;
	},
	setRefreshedAt(next: number | null) {
		refreshedAt = next;
	},
	reset() {
		rows = [];
		selectedIndex = null;
		detail = null;
		detailError = null;
		error = null;
		complete = true;
		refreshedAt = null;
		version = 0;
		calls.ensured = [];
		calls.selected = [];
	}
};

export const graph = {
	get version() {
		return version;
	},
	get count() {
		return rows.length;
	},
	/** The real store's figure: lanes over every row, not the rows on screen. */
	get lanes() {
		return lanesNeeded(0, rows.length - 1, (i) => rows[i]);
	},
	get complete() {
		return complete;
	},
	get refreshedAt() {
		return refreshedAt;
	},
	get error() {
		return error;
	},
	get selectedIndex() {
		return selectedIndex;
	},
	get selected() {
		return selectedIndex === null ? null : (rows[selectedIndex] ?? null);
	},
	get detail() {
		return detail;
	},
	get detailError() {
		return detailError;
	},
	row(index: number): GraphRow | undefined {
		return rows[index];
	},
	ensure(end: number) {
		calls.ensured.push(end);
	},
	select(index: number) {
		calls.selected.push(index);
		selectedIndex = index;
	},
	async attach() {
		return () => {};
	},
	async restart() {},
	async reload() {},
	clear() {
		control.reset();
	}
};
