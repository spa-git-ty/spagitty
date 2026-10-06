// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merger's state (FEAT-100): the two branches, where the result lands, how,
 * and the dry run's forecast for the pair.
 *
 * The forecast is per pair, not per direction: changing where it lands or how
 * re-derives the plan from the same forecast (`plan.ts`) and asks git nothing.
 * Picking another branch, or the refs moving under the screen, asks again.
 * Superseded answers are dropped rather than painted over a newer one.
 */

import * as api from '../api';
import type { BranchRow, MergerForecast, Tag } from '../types';
import {
	defaultNewName,
	effectiveStrategy,
	roles,
	strategies,
	type Into,
	type Roles,
	type Strategy,
	type StrategyChoice
} from './plan';

/** One name a branch picker offers. */
export interface Pickable {
	name: string;
	group: 'local' | 'remote' | 'tag';
	time: number;
}

let a = $state<string | null>(null);
let b = $state<string | null>(null);
let into = $state<Into>('a');
let asked = $state<Strategy>('merge');
/** The new branch's name as typed; null until it is, so it follows A and B. */
let typedName = $state<string | null>(null);

let forecast = $state<MergerForecast | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let pickables = $state<Pickable[]>([]);
let current = $state<string | null>(null);
let seq = 0;

const KEY = 'spagitty.merger.pick';

interface Remembered {
	a: string | null;
	b: string | null;
	into: Into;
	strategy: Strategy;
}

/** Per viewer and per repository: a convenience, so it may be lost. */
function remembered(repo: string): Remembered | null {
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, Remembered>;
		return all[repo] ?? null;
	} catch {
		return null;
	}
}

function remember(repo: string | null) {
	if (!repo) return;
	try {
		const all = JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, Remembered>;
		all[repo] = { a, b, into, strategy: asked };
		localStorage.setItem(KEY, JSON.stringify(all));
	} catch {
		// Storage is unavailable: the pick is only for this visit.
	}
}

let repoPath: string | null = null;

/** Local branches newest first, then remote-tracking branches, then tags. */
export function pickablesFrom(rows: BranchRow[], tags: Tag[]): Pickable[] {
	const local = rows
		.filter((row) => row.kind === 'branch')
		.map((row) => ({ name: row.name, group: 'local' as const, time: row.time }));
	const remote = rows
		.filter((row) => row.kind === 'remote' && !row.name.endsWith('/HEAD'))
		.map((row) => ({ name: row.name, group: 'remote' as const, time: row.time }));
	const tagged = tags.map((tag) => ({ name: tag.name, group: 'tag' as const, time: 0 }));
	const byTime = (x: Pickable, y: Pickable) => y.time - x.time;
	return [...local.sort(byTime), ...remote.sort(byTime), ...tagged];
}

/**
 * The pair to start from: what was picked last time if both still exist,
 * otherwise the checked-out branch and the most recently committed other
 * local branch.
 */
export function startingPair(
	options: Pickable[],
	checkedOut: string | null,
	last: Remembered | null
): { a: string | null; b: string | null } {
	const names = new Set(options.map((option) => option.name));
	if (last?.a && last.b && names.has(last.a) && names.has(last.b) && last.a !== last.b) {
		return { a: last.a, b: last.b };
	}
	const local = options.filter((option) => option.group === 'local');
	const first = checkedOut && names.has(checkedOut) ? checkedOut : (local[0]?.name ?? null);
	const second = local.find((option) => option.name !== first)?.name ?? null;
	return { a: first, b: second };
}

export const merger = {
	get a(): string | null {
		return a;
	},
	get b(): string | null {
		return b;
	},
	get into(): Into {
		return into;
	},
	get forecast(): MergerForecast | null {
		return forecast;
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},
	get pickables(): Pickable[] {
		return pickables;
	},
	/** The branch checked out here, for the pickers to mark. */
	get current(): string | null {
		return current;
	},
	get newName(): string {
		return typedName ?? (a && b ? defaultNewName(a, b) : '');
	},
	get roles(): Roles | null {
		return forecast ? roles(forecast, into, this.newName) : null;
	},
	get choices(): StrategyChoice[] {
		const who = this.roles;
		return forecast && who ? strategies(forecast, who) : [];
	},
	/** The strategy shown: the one asked for, unless it is not possible here. */
	get strategy(): Strategy {
		return effectiveStrategy(this.choices, asked);
	},
	/** Local branch names, for telling a new branch's name is taken. */
	get localNames(): string[] {
		return pickables.filter((option) => option.group === 'local').map((option) => option.name);
	},

	/** Read the names the pickers offer, and settle on a pair. */
	async prime(path: string | null, checkedOut: string | null): Promise<void> {
		const changed = path !== repoPath;
		repoPath = path;
		current = checkedOut;
		if (changed) {
			forecast = null;
			error = null;
			typedName = null;
		}
		const [rows, tags] = await Promise.all([api.branches(), api.tags().catch(() => [] as Tag[])]);
		pickables = pickablesFrom(rows, tags);
		if (changed || a === null || b === null) {
			const last = path ? remembered(path) : null;
			const pair = startingPair(pickables, checkedOut, last);
			a = pair.a;
			b = pair.b;
			if (last) {
				into = last.into;
				asked = last.strategy;
			}
		}
		await this.load();
	},

	/** Ask for the forecast of the pair picked now. */
	async load(): Promise<void> {
		const mine = ++seq;
		if (!a || !b) {
			forecast = null;
			loading = false;
			return;
		}
		if (a === b) {
			forecast = null;
			error = 'Pick two different branches';
			loading = false;
			return;
		}
		loading = true;
		try {
			const next = await api.mergerForecast(a, b);
			if (mine !== seq) return;
			forecast = next;
			error = null;
		} catch (e) {
			if (mine !== seq) return;
			forecast = null;
			error = String(e);
		} finally {
			if (mine === seq) loading = false;
		}
	},

	pickA(name: string) {
		a = name;
		remember(repoPath);
		void this.load();
	},
	pickB(name: string) {
		b = name;
		remember(repoPath);
		void this.load();
	},
	setInto(next: Into) {
		into = next;
		remember(repoPath);
	},
	/** Turn the direction round: into A becomes into B and back. */
	swap() {
		into = into === 'b' ? 'a' : 'b';
		remember(repoPath);
	},
	setStrategy(next: Strategy) {
		asked = next;
		remember(repoPath);
	},
	setNewName(name: string) {
		typedName = name;
	},

	/** For tests and previews: a pair and its forecast, as if read. */
	seed(next: { a: string; b: string; forecast: MergerForecast; pickables?: Pickable[] }) {
		a = next.a;
		b = next.b;
		forecast = next.forecast;
		pickables = next.pickables ?? [];
		error = null;
		loading = false;
	},

	/** For tests: back to nothing picked. */
	reset() {
		a = null;
		b = null;
		into = 'a';
		asked = 'merge';
		typedName = null;
		forecast = null;
		loading = false;
		error = null;
		pickables = [];
		current = null;
		repoPath = null;
		seq += 1;
	}
};
