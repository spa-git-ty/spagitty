// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A reactive stand-in for the branch store, for component tests.
 *
 * Reactive for the same reason as the repository stub: the toolbar's dropdown
 * fills in after `load()` resolves, and a stub built from plain properties
 * would render the empty list and then never change.
 *
 * Lives outside `src/lib` so it is not counted as first-party code by the
 * coverage floor.
 */

import type { BranchRow } from '$lib/types';

let rows = $state<BranchRow[]>([]);
let loaded = $state(false);
let loading = $state(false);
let writeError = $state<string | null>(null);

/** Calls the components made, for assertions. */
export const calls = {
	loads: 0,
	checkedOut: [] as string[]
};

/** Makes the next `checkout` report failure, with this message. */
let checkoutFails: string | null = null;

export const control = {
	setRows(next: BranchRow[]) {
		rows = next;
		loaded = true;
	},
	/** Not loaded yet: what the dropdown shows before the first read returns. */
	setUnloaded() {
		rows = [];
		loaded = false;
	},
	setLoading(next: boolean) {
		loading = next;
	},
	failNextCheckout(message: string) {
		checkoutFails = message;
	},
	reset() {
		rows = [];
		loaded = false;
		loading = false;
		writeError = null;
		checkoutFails = null;
		calls.loads = 0;
		calls.checkedOut = [];
	}
};

/** One row, with the fields the toolbar reads and defaults for the rest. */
export function branchRow(overrides: Partial<BranchRow> = {}): BranchRow {
	return {
		name: 'main',
		fullName: 'refs/heads/main',
		kind: 'branch',
		current: false,
		id: 'a'.repeat(40),
		short: 'aaaaaaa',
		summary: 'a commit',
		authorName: 'someone',
		time: 0,
		upstream: null,
		ahead: null,
		behind: null,
		merged: false,
		...overrides
	};
}

export const branches = {
	get rows(): BranchRow[] {
		return rows;
	},
	get loaded(): boolean {
		return loaded;
	},
	get loading(): boolean {
		return loading;
	},
	get writeError(): string | null {
		return writeError;
	},

	async load(): Promise<void> {
		calls.loads += 1;
		loaded = true;
	},

	async checkout(name: string): Promise<boolean> {
		calls.checkedOut.push(name);
		if (checkoutFails === null) {
			writeError = null;
			return true;
		}
		writeError = checkoutFails;
		checkoutFails = null;
		return false;
	}
};
