// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Merger's state (FEAT-100): the two branches, where the result lands, how,
 * and the dry run's forecast for the pair.
 *
 * The forecast is per pair, not per direction: changing where it lands or how
 * re-derives the plan from the same forecast (`plan.ts`) and asks git nothing.
 * Picking another branch, or the refs moving under the screen, asks again.
 * Superseded answers are dropped rather than painted over a newer one.
 *
 * Landing (FEAT-101) goes through the commit dialog, from the plan when there
 * is nothing to resolve: it names what will be written and takes the message,
 * and only its button writes.
 */

import * as api from '../api';
import type { BranchRow, MergerForecast, MergerLandAsk, MergerLanded, MergerResolution, Tag } from '../types';
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

/** Where the screen is: the plan, resolving, the commit dialog, or done. */
export type Phase = 'plan' | 'resolve' | 'commit' | 'done';
let phase = $state<Phase>('plan');
/** Where Back from the commit dialog goes. */
let returnTo = $state<'plan' | 'resolve'>('plan');
/** The message as typed; null until it is, so it follows the plan. */
let typedMessage = $state<string | null>(null);
let landing = $state(false);
let landError = $state<string | null>(null);
/** What landed, and what it came from, for the done state. */
let landed = $state<{ landed: MergerLanded; source: string; strategy: Strategy } | null>(null);

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
/** A pair asked for from elsewhere — the graph's drag — taken on the next prime. */
let preset: { a: string; b: string; into: Into } | null = null;

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
	get phase(): Phase {
		return phase;
	},
	get landing(): boolean {
		return landing;
	},
	/** Where the commit dialog was opened from. */
	get returnTo(): 'plan' | 'resolve' {
		return returnTo;
	},
	get landError(): string | null {
		return landError;
	},
	get landed() {
		return landed;
	},
	/** `Merge branch 'feat' into main`, or `Squash feat into main`. */
	get defaultMessage(): string {
		const who = this.roles;
		if (!who) return '';
		return this.strategy === 'squash'
			? `Squash ${who.sourceName} into ${who.targetName}`
			: `Merge branch '${who.sourceName}' into ${who.targetName}`;
	},
	get message(): string {
		return typedMessage ?? this.defaultMessage;
	},
	setMessage(text: string) {
		typedMessage = text;
	},

	/** Open the commit dialog, from the plan or from resolving. */
	openCommit(from: 'plan' | 'resolve' = 'plan') {
		returnTo = from;
		landError = null;
		phase = 'commit';
	},
	/** Back out of the commit dialog, writing nothing. */
	back() {
		if (landing) return;
		phase = returnTo;
		landError = null;
	},
	setPhase(next: Phase) {
		phase = next;
	},

	/** What the backend needs to land or replay the merge planned now. */
	ask(resolutions: MergerResolution[] = []): MergerLandAsk | null {
		const plan = forecast;
		if (!plan || !a || !b) return null;
		const strategy = this.strategy;
		return {
			a,
			b,
			aTip: plan.a.tip,
			bTip: plan.b.tip,
			target: into,
			newName: into === 'new' ? this.newName : undefined,
			strategy,
			message: strategy === 'rebase' || strategy === 'ff' ? undefined : this.message,
			resolutions
		};
	},

	/**
	 * Write it: commit the result and move the receiving branch. Refused by the
	 * backend if either branch moved since the forecast was read.
	 */
	async land(resolutions: MergerResolution[] = []): Promise<boolean> {
		const ask = this.ask(resolutions);
		const who = this.roles;
		if (!ask || !who || landing) return false;
		// A rebase lands from its own worktree once every commit is replayed
		// (FEAT-103), not in one call.
		const rebase = ask.strategy === 'rebase';
		landing = true;
		landError = null;
		try {
			const result = rebase ? await api.mergerRebaseFinish(ask) : await api.mergerLand(ask);
			landed = { landed: result, source: who.sourceName, strategy: ask.strategy };
			phase = 'done';
			return true;
		} catch (e) {
			landError = String(e);
			return false;
		} finally {
			landing = false;
		}
	},

	/** Merge another: back to the plan, read afresh. */
	again() {
		phase = 'plan';
		landed = null;
		typedMessage = null;
		landError = null;
		void this.load();
	},

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
		const wanted = preset;
		preset = null;
		if (wanted) {
			a = wanted.a;
			b = wanted.b;
			into = wanted.into;
			remember(path);
		} else if (changed || a === null || b === null) {
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

	/**
	 * Open on this pair next time the screen primes (TASK-056): what the
	 * graph's drag of one branch onto another asks for, B coming into A.
	 */
	present(next: { a: string; b: string; into: Into }) {
		preset = next;
		phase = 'plan';
		landed = null;
		typedMessage = null;
		typedName = null;
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
		preset = null;
		phase = 'plan';
		returnTo = 'plan';
		typedMessage = null;
		landing = false;
		landError = null;
		landed = null;
		seq += 1;
	}
};
