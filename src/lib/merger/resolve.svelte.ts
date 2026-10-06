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
import type { MergerConflicts, MergerResolution } from '../types';
import type { SummaryRow } from './plan';
import { merger } from './store.svelte';

let data = $state<MergerConflicts | null>(null);
let files = $state<ResolverFile[]>([]);
let choices = $state<Record<string, (Choice | null)[]>>({});
let loading = $state(false);
let error = $state<string | null>(null);
let start = $state<string | null>(null);
let key: string | null = null;
let seq = 0;
let saving: ReturnType<typeof setTimeout> | null = null;

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

interface Kept {
	version: 1;
	choices: Record<string, Record<string, { fp: string; choice: Choice }>>;
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
		const made: Record<string, { fp: string; choice: Choice }> = {};
		file.regions.forEach((region) => {
			const choice = choices[file.path]?.[region.index];
			if (choice) made[String(region.index)] = { fp: fingerprint(region), choice };
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
		} catch (e) {
			if (mine === seq) error = String(e);
		} finally {
			if (mine === seq) loading = false;
		}
	},

	choose(path: string, index: number, choice: Choice | null) {
		const row = [...(choices[path] ?? [])];
		row[index] = choice;
		choices = { ...choices, [path]: row };
		persist();
	},

	/** All from one side, for every region of a file. */
	whole(path: string, side: SideKey) {
		const file = files.find((f) => f.path === path);
		if (!file) return;
		choices = { ...choices, [path]: file.regions.map(() => ({ mode: side })) };
		persist();
	},

	/** Back to the plan, keeping the choices. */
	toPlan() {
		merger.setPhase('plan');
	},

	/** Abort: the choices are dropped and forgotten, and nothing was written. */
	abort() {
		choices = Object.fromEntries(files.map((file) => [file.path, file.regions.map(() => null)]));
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
				return {
					where: `Conflict ${n} · ${name}:${lineOf(region, places[k])}`,
					label: statusLabel(choice, names),
					badges: badgesOf(choice),
					mine: choice?.mode === 'edit'
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
			key = null;
		}
		return landed;
	},

	/** For tests and previews: conflicts as if read, and the choices so far. */
	seed(found: MergerConflicts, made: Record<string, (Choice | null)[]> = {}) {
		data = found;
		files = filesFrom(found);
		choices = { ...restore(null, files), ...made };
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
		loading = false;
		error = null;
		start = null;
		key = null;
		seq += 1;
	}
};
