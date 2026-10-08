// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Resolving a Merger merge (FEAT-102): the conflicts of the planned pair,
 * read from a dry run, and the choices made for each.
 *
 * Nothing is written while resolving. The choices are kept in Spagitty's
 * application data, one file per merge — keyed by the repository, both
 * branches and their merge base — so leaving the screen and coming back finds
 * them where they were. A choice kept is only applied to a region whose sides
 * are the same as when it was made; one that has moved is left unresolved
 * rather than applied to the wrong lines.
 */

import * as api from '../api';
import {
	badgesOf,
	counts,
	fingerprint,
	hash,
	landing,
	layout,
	lineOf,
	readFile,
	statusLabel,
	type Choice,
	type Names,
	type ResolverFile,
	type SideKey
} from '../resolver/model';
import type { MergerCommit, MergerConflicts, MergerReplay, MergerResolution } from '../types';
import type { SummaryRow } from './plan';
import { merger } from './store.svelte';

let data = $state<MergerConflicts | null>(null);
let files = $state<ResolverFile[]>([]);
let choices = $state<Record<string, (Choice | null)[]>>({});
/**
 * Who chose each region, when an agent did (2.0): its name, and whether it
 * applied the choice itself or the person accepted it. Absent is the person.
 * Kept beside the choice, and dropped when the person chooses again.
 */
let authors = $state<Record<string, (Author | null)[]>>({});
let loading = $state(false);
let error = $state<string | null>(null);
let start = $state<string | null>(null);
let key: string | null = null;
let seq = 0;
let saving: ReturnType<typeof setTimeout> | null = null;
/** Where Merger's rebase stopped, while one is stopped (FEAT-103). */
let stop = $state<{ step: number; total: number; commit: MergerCommit | null } | null>(null);
let stepping = $state(false);

/** The resolver's files, from the backend's. */
export function filesFrom(found: MergerConflicts): ResolverFile[] {
	return found.files.map((file) =>
		readFile({
			path: file.path,
			kind: file.kind,
			merged: file.merged?.text ?? null,
			a: file.a && !file.a.binary && !file.a.tooLarge ? file.a.text : null,
			b: file.b && !file.b.binary && !file.b.tooLarge ? file.b.text : null,
			aExists: file.a !== null,
			bExists: file.b !== null,
			opaque: Boolean(file.a?.binary || file.b?.binary || file.a?.tooLarge || file.b?.tooLarge),
			sources: file.regions.map((region) => ({
				index: region.index,
				aLine: region.aLine,
				bLine: region.bLine,
				aFrom: region.aCommit ? `${region.aCommit.short} · ${region.aCommit.summary}` : null,
				bFrom: region.bCommit ? `${region.bCommit.short} · ${region.bCommit.summary}` : null
			}))
		})
	);
}

/** Sixteen hex digits naming one merge: the repository, both names, the base. */
export function keyOf(repo: string, a: string, b: string, base: string): string {
	const text = [repo, a, b, base].join('\u0000');
	return hash(text) + hash(`merger\u0000${text}`);
}

/** An agent that chose a region, and how it came to stand. */
export interface Author {
	agent: string;
	/** `agent`: applied by the agent itself; `person`: accepted by the person. */
	decided: 'agent' | 'person';
	/** Unattended: the agent also landed it. */
	unattended?: boolean;
}

interface Kept {
	version: 1;
	choices: Record<string, Record<string, { fp: string; choice: Choice; by?: Author }>>;
}

function author(value: unknown): Author | null {
	if (!value || typeof value !== 'object') return null;
	const raw = value as Record<string, unknown>;
	if (typeof raw.agent !== 'string' || (raw.decided !== 'agent' && raw.decided !== 'person')) return null;
	return { agent: raw.agent, decided: raw.decided, unattended: raw.unattended === true };
}

/** The kept authors that still fit these files: only where the choice did. */
export function restoreAuthors(saved: unknown, against: ResolverFile[]): Record<string, (Author | null)[]> {
	const out: Record<string, (Author | null)[]> = {};
	const kept = (saved as Kept | null)?.version === 1 ? (saved as Kept).choices : null;
	for (const file of against) {
		out[file.path] = file.regions.map((region) => {
			const entry = kept?.[file.path]?.[String(region.index)];
			if (!entry || entry.fp !== fingerprint(region) || !valid(entry.choice, region)) return null;
			return author(entry.by);
		});
	}
	return out;
}

/** `you`, `Codex, accepted by you`, `Codex, unattended`. */
export function whoChose(by: Author | null): string {
	if (!by) return 'you';
	if (by.decided === 'person') return `${by.agent}, accepted by you`;
	return by.unattended ? `${by.agent}, unattended` : `${by.agent}, applied at Sign off`;
}

function valid(choice: unknown, region: ResolverFile['regions'][number]): choice is Choice {
	if (!choice || typeof choice !== 'object') return false;
	const c = choice as Record<string, unknown>;
	switch (c.mode) {
		case 'a':
		case 'b':
		case 'ab':
		case 'ba':
			return true;
		case 'pick':
			return (
				Array.isArray(c.a) &&
				Array.isArray(c.b) &&
				c.a.length === region.a.length &&
				c.b.length === region.b.length
			);
		case 'edit':
			return typeof c.text === 'string';
		default:
			return false;
	}
}

/** The kept choices that still fit these files. */
export function restore(saved: unknown, against: ResolverFile[]): Record<string, (Choice | null)[]> {
	const out: Record<string, (Choice | null)[]> = {};
	const kept = (saved as Kept | null)?.version === 1 ? (saved as Kept).choices : null;
	for (const file of against) {
		out[file.path] = file.regions.map((region) => {
			const entry = kept?.[file.path]?.[String(region.index)];
			if (!entry || entry.fp !== fingerprint(region) || !valid(entry.choice, region)) return null;
			return entry.choice;
		});
	}
	return out;
}

function serialise(): Kept {
	const out: Kept = { version: 1, choices: {} };
	for (const file of files) {
		const made: Record<string, { fp: string; choice: Choice; by?: Author }> = {};
		file.regions.forEach((region) => {
			const choice = choices[file.path]?.[region.index];
			const by = authors[file.path]?.[region.index] ?? null;
			if (choice) made[String(region.index)] = { fp: fingerprint(region), choice, ...(by ? { by } : {}) };
		});
		if (Object.keys(made).length > 0) out.choices[file.path] = made;
	}
	return out;
}

function persist() {
	if (!key || !api.inTauri()) return;
	const target = key;
	if (saving) clearTimeout(saving);
	saving = setTimeout(() => {
		saving = null;
		api.setMergerState(target, serialise()).catch(() => {
			// Not kept: the choices are still on screen, and still land.
		});
	}, 300);
}

function forget() {
	if (saving) clearTimeout(saving);
	saving = null;
	if (key && api.inTauri()) api.setMergerState(key, null).catch(() => {});
}

export const resolving = {
	get files(): ResolverFile[] {
		return files;
	},
	get choices(): Record<string, (Choice | null)[]> {
		return choices;
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},
	get start(): string | null {
		return start;
	},
	get baseShort(): string | null {
		return data?.baseShort ?? null;
	},
	get counts(): { total: number; resolved: number } {
		return counts(files, choices);
	},
	/** Regions still without a choice; 0 when nothing is being resolved. */
	get unresolved(): number {
		if (merger.phase !== 'resolve' && merger.phase !== 'commit') return 0;
		const { total, resolved } = this.counts;
		return total - resolved;
	},
	/** The rebase's stop, while one is stopped. */
	get stop() {
		return stop;
	},
	get stepping(): boolean {
		return stepping;
	},

	/**
	 * Start the rebase, or find where it stopped (FEAT-103). Done at once —
	 * nothing in the way — goes straight to the commit dialog; a stop is
	 * resolved here, one commit at a time.
	 */
	async openRebase(): Promise<void> {
		const ask = merger.ask();
		if (!ask) return;
		const mine = ++seq;
		loading = true;
		error = null;
		try {
			const replay = await api.mergerRebaseOpen(ask);
			if (mine === seq) this.follow(replay);
		} catch (e) {
			if (mine === seq) {
				error = String(e);
				merger.setPhase('resolve');
			}
		} finally {
			if (mine === seq) loading = false;
		}
	},

	/** Show where the rebase is now: its next stop, or the commit dialog. */
	follow(replay: MergerReplay) {
		if (replay.state === 'done') {
			stop = null;
			files = [];
			choices = {};
			// The dialog opens over the plan: the stops are behind it now.
			merger.openCommit('plan');
			return;
		}
		stop = { step: replay.step, total: replay.total, commit: replay.commit };
		data = null;
		key = null;
		files = filesFrom({ base: '', baseShort: '', aTip: '', bTip: '', files: replay.files });
		choices = restore(null, files);
		start = null;
		merger.setPhase('resolve');
	},

	/** Settle this stop with what was chosen, and carry the rebase on. */
	async continueRebase(): Promise<void> {
		const ask = merger.ask();
		const resolutions = this.resolutions();
		if (!ask || !resolutions || stepping) return;
		stepping = true;
		error = null;
		try {
			this.follow(await api.mergerRebaseContinue(ask, resolutions));
		} catch (e) {
			error = String(e);
		} finally {
			stepping = false;
		}
	},

	/** Drop the commit the rebase stopped on. */
	async skipRebase(): Promise<void> {
		const ask = merger.ask();
		if (!ask || stepping) return;
		stepping = true;
		error = null;
		try {
			this.follow(await api.mergerRebaseSkip(ask));
		} catch (e) {
			error = String(e);
		} finally {
			stepping = false;
		}
	},

	/** Undo the rebase; back to the plan, and neither branch moved. */
	async abortRebase(): Promise<void> {
		const ask = merger.ask();
		if (!ask || stepping) return;
		stepping = true;
		error = null;
		try {
			await api.mergerRebaseAbort(ask);
			stop = null;
			files = [];
			choices = {};
			merger.setPhase('plan');
		} catch (e) {
			error = String(e);
		} finally {
			stepping = false;
		}
	},

	get ready(): boolean {
		const { total, resolved } = this.counts;
		return total > 0 && resolved === total;
	},

	/** Open resolving at `path`, or the first conflict. */
	async open(repo: string, path: string | null): Promise<void> {
		const a = merger.a;
		const b = merger.b;
		const plan = merger.forecast;
		if (!a || !b || !plan) return;
		const mine = ++seq;
		start = path;
		stop = null;
		merger.setPhase('resolve');
		const nextKey = keyOf(repo, a, b, plan.base);
		if (key === nextKey && data && data.aTip === plan.a.tip && data.bTip === plan.b.tip) return;
		loading = true;
		error = null;
		try {
			const found = await api.mergerConflicts(a, b);
			if (mine !== seq) return;
			const read = filesFrom(found);
			const saved = api.inTauri() ? await api.mergerState(nextKey).catch(() => null) : null;
			if (mine !== seq) return;
			data = found;
			files = read;
			key = nextKey;
			choices = restore(saved, read);
			authors = restoreAuthors(saved, read);
		} catch (e) {
			if (mine === seq) error = String(e);
		} finally {
			if (mine === seq) loading = false;
		}
	},

	/** Choose for a region. `by` is the agent that chose it; absent is the person. */
	choose(path: string, index: number, choice: Choice | null, by: Author | null = null) {
		const row = [...(choices[path] ?? [])];
		row[index] = choice;
		choices = { ...choices, [path]: row };
		const who = [...(authors[path] ?? [])];
		who[index] = choice ? by : null;
		authors = { ...authors, [path]: who };
		persist();
	},

	/** Who chose each region, where an agent did (2.0). */
	get authors(): Record<string, (Author | null)[]> {
		return authors;
	},

	/** Every agent whose choice stands, for the commit's trailer. */
	get agentsInResult(): string[] {
		const names = new Set<string>();
		for (const file of files) {
			file.regions.forEach((region) => {
				const by = authors[file.path]?.[region.index];
				if (by && choices[file.path]?.[region.index]) names.add(by.agent);
			});
		}
		return [...names];
	},

	/** Mark what an agent landed itself as unattended, for the record. */
	markUnattended(agent: string) {
		authors = Object.fromEntries(
			Object.entries(authors).map(([path, row]) => [
				path,
				row.map((by) => (by && by.agent === agent && by.decided === 'agent' ? { ...by, unattended: true } : by))
			])
		);
	},

	/** All from one side, for every region of a file. */
	whole(path: string, side: SideKey) {
		const file = files.find((f) => f.path === path);
		if (!file) return;
		choices = { ...choices, [path]: file.regions.map(() => ({ mode: side })) };
		authors = { ...authors, [path]: file.regions.map(() => null) };
		persist();
	},

	/** Back to the plan, keeping the choices. */
	toPlan() {
		merger.setPhase('plan');
	},

	/** Abort: the choices are dropped and forgotten, and nothing was written. */
	abort() {
		choices = Object.fromEntries(files.map((file) => [file.path, file.regions.map(() => null)]));
		authors = {};
		forget();
		merger.setPhase('plan');
	},

	/** What lands at each conflicted path, or null while any is unresolved. */
	resolutions(): MergerResolution[] | null {
		const out: MergerResolution[] = [];
		for (const file of files) {
			const lands = landing(file, choices[file.path] ?? []);
			if (!lands) return null;
			out.push(lands);
		}
		return out;
	},

	/** Each conflict and what was chosen for it, for the commit dialog. */
	summary(names: Names): SummaryRow[] {
		let n = 0;
		return files.flatMap((file) => {
			const places = layout(file, choices[file.path] ?? []);
			const name = file.path.slice(file.path.lastIndexOf('/') + 1);
			return file.regions.map((region, k) => {
				n += 1;
				const choice = choices[file.path]?.[region.index] ?? null;
				const by = authors[file.path]?.[region.index] ?? null;
				return {
					where: `Conflict ${n} · ${name}:${lineOf(region, places[k])}`,
					label: statusLabel(choice, names),
					badges: badgesOf(choice),
					mine: choice?.mode === 'edit' && !by,
					who: whoChose(by)
				};
			});
		});
	},

	/** Land it, and forget the kept choices once it has. */
	async commit(): Promise<boolean> {
		const resolutions = this.resolutions();
		if (!resolutions) return false;
		const landed = await merger.land(resolutions);
		if (landed) {
			forget();
			data = null;
			files = [];
			choices = {};
			authors = {};
			key = null;
		}
		return landed;
	},

	/** For tests and previews: conflicts as if read, and the choices so far. */
	seed(found: MergerConflicts, made: Record<string, (Choice | null)[]> = {}) {
		data = found;
		files = filesFrom(found);
		choices = { ...restore(null, files), ...made };
		authors = {};
		loading = false;
		error = null;
	},

	/** For tests. */
	reset() {
		if (saving) clearTimeout(saving);
		saving = null;
		data = null;
		files = [];
		choices = {};
		authors = {};
		loading = false;
		error = null;
		start = null;
		key = null;
		stop = null;
		stepping = false;
		seq += 1;
	}
};
