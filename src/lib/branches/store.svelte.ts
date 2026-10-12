// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Every branch, and what is worth knowing about each one.
 *
 * The whole list arrives in one call — a repository with a thousand branches is
 * a rounding error next to its history — so filtering happens here rather than
 * by asking the core again for a narrower question.
 */

import * as api from '../api';
import { activity, type ActivityKind } from '../activity/store.svelte';
import { repo } from '../repo.svelte';
import type { BranchRow } from '../types';
import { branchDeleted } from '$lib/delight/watch';

/**
 * How long a branch has to sit untouched before it is "stale".
 *
 * Ninety days is a quarter: long enough that a branch someone is actually
 * working on is never labelled abandoned, short enough to catch the ones that
 * were.
 */
export const STALE_DAYS = 90;

/**
 * The filter chips, in the order the screen draws them.
 *
 * `mine` means "local", not "authored by me": Spagitty does not know who you are
 * until Settings does, and a chip that guessed would be wrong for anyone who
 * commits under more than one identity.
 */
export const FILTERS = ['mine', 'origin', 'upstream', 'merged', 'stale'] as const;
export type Filter = (typeof FILTERS)[number];

let rows = $state<BranchRow[]>([]);
let loaded = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);

let query = $state('');
let active = $state<Filter[]>([]);

let busy = $state(false);
let writeError = $state<string | null>(null);

/** The name a new branch would be created under, and where from. */
let newName = $state('');
let newStart = $state('');
let newCheckout = $state(true);

let seq = 0;

function matches(row: BranchRow, filter: Filter, now: number): boolean {
	switch (filter) {
		case 'mine':
			return row.kind === 'branch';
		case 'origin':
			return row.kind === 'remote';
		case 'upstream':
			return row.upstream !== null;
		case 'merged':
			return row.merged;
		case 'stale':
			return now / 1000 - row.time > STALE_DAYS * 86400;
	}
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
	get error(): string | null {
		return error;
	},
	get query(): string {
		return query;
	},
	get active(): Filter[] {
		return active;
	},
	get busy(): boolean {
		return busy;
	},
	get writeError(): string | null {
		return writeError;
	},
	get newName(): string {
		return newName;
	},
	get newStart(): string {
		return newStart;
	},
	get newCheckout(): boolean {
		return newCheckout;
	},

	/**
	 * The rows the screen shows.
	 *
	 * Chips compose as AND, which is what makes "merged" and "stale" together
	 * mean what it looks like it means. The text filter matches the branch name
	 * and its upstream, since "origin/ma" is a reasonable thing to type.
	 */
	get filtered(): BranchRow[] {
		const needle = query.trim().toLowerCase();
		const now = Date.now();

		return rows.filter((row) => {
			if (needle !== '') {
				const haystack = `${row.name} ${row.upstream ?? ''}`.toLowerCase();
				if (!haystack.includes(needle)) return false;
			}
			return active.every((filter) => matches(row, filter, now));
		});
	},

	/** How many rows the filters are hiding, so the screen can say so. */
	get hidden(): number {
		return rows.length - this.filtered.length;
	},

	get current(): BranchRow | null {
		return rows.find((row) => row.current) ?? null;
	},

	setQuery(next: string) {
		query = next;
	},

	toggle(filter: Filter) {
		active = active.includes(filter)
			? active.filter((f) => f !== filter)
			: [...active, filter];
	},

	clearFilters() {
		active = [];
		query = '';
	},

	setNewName(next: string) {
		newName = next;
	},
	setNewStart(next: string) {
		newStart = next;
	},
	setNewCheckout(next: boolean) {
		newCheckout = next;
	},

	async load(): Promise<void> {
		loading = true;
		const current = ++seq;
		try {
			const next = await api.branches();
			if (current !== seq) return;
			rows = next;
			loaded = true;
			error = null;
		} catch (e) {
			if (current === seq) {
				error = String(e);
				rows = [];
			}
		} finally {
			if (current === seq) loading = false;
		}
	},

	/**
	 * Run a write, then re-read the list and the repository's own state.
	 *
	 * `doing` is what the toolbar says while it runs (FEAT-121). A write that
	 * would collide with one still running is not started, and says which.
	 */
	async run(
		operation: () => Promise<void>,
		doing = 'Changing the branches',
		kind: ActivityKind = 'branch'
	): Promise<boolean> {
		if (busy) return false;
		const step = activity.begin(kind, doing);
		if (!step) {
			writeError = activity.blocked('repository');
			return false;
		}
		busy = true;
		writeError = null;
		try {
			await operation();
			return true;
		} catch (e) {
			writeError = String(e);
			return false;
		} finally {
			busy = false;
			step.end();
			await this.load();
			await repo.refresh();
		}
	},

	/** Check out a branch. Refused by git if it would overwrite work. */
	checkout(name: string): Promise<boolean> {
		return this.run(() => api.checkout(name), `Switching to ${name}`, 'checkout');
	},

	/**
	 * Rename a local branch.
	 *
	 * `git branch -m` carries the upstream configuration and the reflog across
	 * with the ref, so there is nothing to move afterwards. A remote that
	 * already has the old name keeps it — the confirmation says so.
	 */
	rename(from: string, to: string): Promise<boolean> {
		if (to.trim() === '' || to === from) return Promise.resolve(false);
		return this.run(() => api.renameBranch(from, to.trim()), `Renaming ${from}`);
	},

	/**
	 * Delete a local branch. `force` is `git branch -D` and loses commits.
	 *
	 * No confirmation here, the same split the rest of the application uses:
	 * `$lib/branches/actions.ts` owns the wording, because what has to be said
	 * depends on whether the branch is merged and on what it would take to get
	 * it back.
	 */
	async delete(name: string, force: boolean): Promise<boolean> {
		const gone = await this.run(() => api.deleteBranch(name, force), `Deleting ${name}`);
		if (gone) branchDeleted(name);
		return gone;
	},

	/**
	 * Delete several branches in one pass.
	 *
	 * Sequential rather than concurrent: `git branch -d` takes a lock, and the
	 * failures are per branch — one that refuses must not take the rest with
	 * it. The first failure stops the run and is reported, because carrying on
	 * past one would leave the user reading a list of things that did not
	 * happen for reasons they never saw.
	 */
	deleteMany(names: string[], force: boolean): Promise<boolean> {
		if (names.length === 0) return Promise.resolve(false);
		return this.run(
			async () => {
				for (const name of names) {
					await api.deleteBranch(name, force);
				}
			},
			names.length === 1 ? `Deleting ${names[0]}` : `Deleting ${names.length} branches`
		);
	},

	/** Create the branch the form describes. Clears the form on success. */
	async create(): Promise<boolean> {
		const name = newName.trim();
		if (name === '' || busy) return false;

		const made = await this.run(
			() => api.createBranch(name, newStart.trim(), newCheckout),
			`Creating ${name}`
		);
		if (made) {
			newName = '';
			newStart = '';
		}
		return made;
	},

	clear(): void {
		seq += 1;
		rows = [];
		loaded = false;
		loading = false;
		error = null;
		query = '';
		active = [];
		busy = false;
		writeError = null;
		newName = '';
		newStart = '';
		newCheckout = true;
	}
};
