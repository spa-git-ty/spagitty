// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A repository stopped mid-operation (FEAT-008), resolved with the shared
 * three-column resolver (FEAT-102).
 *
 * Each conflicted file is read whole — the index's stages and the file on
 * disk with its markers — and every region is a choice made on screen: A, B,
 * both in either order, picked lines or typed text. Nothing reaches the file
 * or the index until *Mark resolved*, which writes what was chosen and stages
 * it in one go. That press is the person saying they looked at the result;
 * nothing marks a file resolved on their behalf.
 *
 * Here A is ours (`HEAD`) and B is theirs, the side coming in. Choices live
 * only on screen: the index already remembers what is still unresolved, and
 * a choice for a file that has since changed on disk is dropped rather than
 * applied to lines it was not made for.
 */

import * as api from '../api';
import { fingerprint, landing, readFile, type Choice, type ResolverFile, type SideKey } from '../resolver/model';
import type { ConflictFile, ConflictOperation, ConflictSides } from '../types';

let operation = $state<ConflictOperation>('none');
let files = $state<ConflictFile[]>([]);
let loaded = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);
let read = $state<ResolverFile[]>([]);
let choices = $state<Record<string, (Choice | null)[]>>({});
/** Files whose markers are gone from disk: they can be marked resolved as they are. */
let settledOnDisk = $state<Record<string, string>>({});
/** Set while a write is in flight, so nothing is done twice. */
let busy = $state(false);
/** What the last write failed with. Cleared when the next one starts. */
let writeError = $state<string | null>(null);
let seq = 0;

/** What each operation is called where the screen has to name it. */
const OPERATION_LABELS: Record<ConflictOperation, string> = {
	merge: 'merge',
	rebase: 'rebase',
	rebaseInteractive: 'interactive rebase',
	cherryPick: 'cherry-pick',
	revert: 'revert',
	applyMailbox: 'patch application',
	bisect: 'bisect',
	none: 'none'
};

/** The resolver's file, from the index's stages and what is on disk. */
export function fileFrom(sides: ConflictSides): ResolverFile {
	const text = (side: ConflictSides['ours']) => (side && !side.binary && !side.tooLarge ? side.text : null);
	return readFile({
		path: sides.path,
		kind: sides.kind,
		merged: text(sides.merged),
		a: text(sides.ours),
		b: text(sides.theirs),
		aExists: sides.ours !== null,
		bExists: sides.theirs !== null,
		opaque: Boolean(sides.ours?.binary || sides.theirs?.binary || sides.ours?.tooLarge || sides.theirs?.tooLarge)
	});
}

/** The choices made before a re-read that still fit the file as it is now. */
function carried(previous: ResolverFile | undefined, next: ResolverFile): (Choice | null)[] {
	const made = choices[next.path] ?? [];
	return next.regions.map((region) => {
		const before = previous?.regions[region.index];
		if (!before || fingerprint(before) !== fingerprint(region)) return null;
		return made[region.index] ?? null;
	});
}

export const conflicts = {
	get operation(): ConflictOperation {
		return operation;
	},
	get operationLabel(): string {
		return OPERATION_LABELS[operation];
	},
	get files(): ConflictFile[] {
		return files;
	},
	/** The conflicted files, read whole, for the resolver. */
	get read(): ResolverFile[] {
		return read;
	},
	get choices(): Record<string, (Choice | null)[]> {
		return choices;
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
	get busy(): boolean {
		return busy;
	},
	get writeError(): string | null {
		return writeError;
	},
	/** True when every conflicted path has been marked resolved. */
	get allResolved(): boolean {
		return loaded && files.length === 0 && operation !== 'none';
	},

	async load(): Promise<void> {
		loading = true;
		const current = ++seq;
		try {
			const next = await api.conflicts();
			const sides = await Promise.all(next.files.map((file) => api.conflictSides(file.path)));
			if (current !== seq) return;
			const fresh = sides.map(fileFrom);
			const previous = new Map(read.map((file) => [file.path, file]));
			choices = Object.fromEntries(fresh.map((file) => [file.path, carried(previous.get(file.path), file)]));
			settledOnDisk = Object.fromEntries(
				sides.flatMap((side, k) =>
					fresh[k].whole && side.merged && !side.merged.binary && side.ours && side.theirs
						? [[side.path, side.merged.text]]
						: []
				)
			);
			operation = next.operation;
			files = next.files;
			read = fresh;
			loaded = true;
			error = null;
		} catch (e) {
			if (current === seq) {
				error = String(e);
				files = [];
				read = [];
				operation = 'none';
			}
		} finally {
			if (current === seq) loading = false;
		}
	},

	choose(path: string, index: number, choice: Choice | null) {
		const row = [...(choices[path] ?? [])];
		row[index] = choice;
		choices = { ...choices, [path]: row };
	},

	whole(path: string, side: SideKey) {
		const file = read.find((f) => f.path === path);
		if (file) choices = { ...choices, [path]: file.regions.map(() => ({ mode: side })) };
	},

	/** A file has a choice for every region, or no markers left on disk. */
	settleable(path: string): boolean {
		const file = read.find((f) => f.path === path);
		if (!file) return false;
		return landing(file, choices[path] ?? []) !== null || path in settledOnDisk;
	},

	/** The file's markers are gone from disk: it can be marked resolved as it is. */
	asOnDisk(path: string): boolean {
		const file = read.find((f) => f.path === path);
		return path in settledOnDisk && (!file || landing(file, choices[path] ?? []) === null);
	},

	/**
	 * Run a write, then re-read everything it could have changed.
	 *
	 * The list and every file's sides move when a resolution lands, and
	 * re-reading is cheaper than working out which.
	 */
	async run(operation: () => Promise<void>): Promise<boolean> {
		if (busy) return false;
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
			await this.load();
		}
	},

	/** Write what was chosen for a file and stage it: `git add`. */
	settle(path: string): Promise<boolean> {
		const file = read.find((f) => f.path === path);
		if (!file) return Promise.resolve(false);
		const lands = landing(file, choices[path] ?? []);
		if (lands) {
			return this.run(() =>
				'take' in lands ? api.conflictSettle(path, null, lands.take) : api.conflictSettle(path, lands.text, null)
			);
		}
		if (path in settledOnDisk) return this.run(() => api.conflictSettle(path, settledOnDisk[path], null));
		return Promise.resolve(false);
	},

	/** Carry on with the operation. Refused by git while anything is conflicted. */
	continue(): Promise<boolean> {
		return this.run(() => api.conflictContinue());
	},

	/** Abandon it. The confirmation belongs to the caller. */
	abort(): Promise<boolean> {
		choices = {};
		return this.run(() => api.conflictAbort());
	},

	clear(): void {
		seq += 1;
		operation = 'none';
		files = [];
		read = [];
		choices = {};
		settledOnDisk = {};
		loaded = false;
		loading = false;
		error = null;
		busy = false;
		writeError = null;
	}
};
