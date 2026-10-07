// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Planning a history rewrite.
 *
 * The edits *are* the state. There is no separate model of "what the result
 * looks like" — the preview is recomputed from the plan after every change, so
 * the two cannot disagree about what would happen.
 *
 * Since FEAT-015 it also runs the plan. Execution is kept at arm's length from
 * planning even so: `run` starts a worker and returns, and everything after
 * that arrives as an event. Three states matter and they are not the same —
 * running, stopped part-way waiting for a person, and finished — and a store
 * that collapsed the middle one into "failed" would send people looking for a
 * problem instead of to the Conflicts screen.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from '../api';
import { repo } from '../repo.svelte';
import { rebaseFinished } from '$lib/delight/watch';
import {
	REBASE_DONE_EVENT,
	REBASE_PROGRESS_EVENT,
	type RebaseAction,
	type RebaseDoneEvent,
	type RebaseEdit,
	type RebasePreview,
	type RebaseProgress,
	type RebaseProgressEvent,
	type RebaseTodo,
	type TodoRow
} from '../types';

let todo = $state<RebaseTodo | null>(null);
let plan = $state<RebaseEdit[]>([]);
let preview = $state<RebasePreview | null>(null);

let upstream = $state('');
let loading = $state(false);
let error = $state<string | null>(null);

/** The row the keyboard is on, by id. Reordering follows it. */
let focused = $state<string | null>(null);

/** Guards against a slow preview landing after a newer edit. */
let previewSeq = 0;

/** How many commits the run in flight was planned to replay (FEAT-072). */
let planned = 0;
/** How many times that run stopped on a conflict. */
let stops = 0;

/** The rebase running right now, by token. Null when none is. */
let token = $state<number | null>(null);
/** Where git has got to, from its own state directory. */
let progress = $state<RebaseProgress | null>(null);
/**
 * What the last run ended as.
 *
 * `stopped` is not a failure: git got part-way and is waiting for a conflict to
 * be resolved or an `edit` to be finished. It is the one outcome that sends the
 * user somewhere else rather than telling them something went wrong.
 */
let outcome = $state<'ran' | 'stopped' | 'failed' | null>(null);
let runError = $state<string | null>(null);
/** Set while continue, skip or abort is in flight. */
let busy = $state(false);

/** The todo rows by id, so a plan entry can find what it is about. */
function rowFor(id: string): TodoRow | null {
	return todo?.rows.find((row) => row.id === id) ?? null;
}

function positionOf(id: string): number {
	return plan.findIndex((entry) => entry.id === id);
}

export const rebase = {
	get todo(): RebaseTodo | null {
		return todo;
	},
	get plan(): RebaseEdit[] {
		return plan;
	},
	get preview(): RebasePreview | null {
		return preview;
	},
	get upstream(): string {
		return upstream;
	},
	set upstream(value: string) {
		upstream = value;
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},
	get focused(): string | null {
		return focused;
	},

	/** True once a todo has been read, so an untouched screen is not "empty". */
	get loaded(): boolean {
		return todo !== null;
	},

	/** The plan as rows, in plan order, each with its action. */
	get rows(): Array<{ row: TodoRow; action: RebaseAction }> {
		const out: Array<{ row: TodoRow; action: RebaseAction }> = [];
		for (const entry of plan) {
			const row = rowFor(entry.id);
			if (row) out.push({ row, action: entry.action });
		}
		return out;
	},

	/** True when the plan differs from the list git would have opened. */
	get edited(): boolean {
		if (!todo) return false;
		if (plan.length !== todo.rows.length) return true;
		return plan.some(
			(entry, at) => entry.action !== 'pick' || entry.id !== todo!.rows[at].id
		);
	},

	rowFor,

	/** Read the todo for `upstream` and start from an unedited plan. */
	async load(): Promise<void> {
		const onto = upstream.trim();
		if (onto === '') return;

		loading = true;
		try {
			const next = await api.rebaseTodo(onto);
			todo = next;
			plan = next.rows.map((row) => ({ id: row.id, action: 'pick' as const }));
			focused = next.rows[0]?.id ?? null;
			error = null;
			await this.recompute();
		} catch (e) {
			error = String(e);
			todo = null;
			plan = [];
			preview = null;
			focused = null;
		} finally {
			loading = false;
		}
	},

	/** Set one row's action. */
	async setAction(id: string, action: RebaseAction): Promise<void> {
		const at = positionOf(id);
		if (at === -1) return;
		// A message typed for a reword survives a change of mind and back.
		plan[at] = { ...plan[at], id, action };
		plan = [...plan];
		await this.recompute();
	},

	/**
	 * The new message for a reword (FEAT-106). Collected on screen, because
	 * the editor git would open at execution time has no terminal here; a
	 * reword with an empty message runs as a pick.
	 */
	setMessage(id: string, message: string): void {
		const at = positionOf(id);
		if (at === -1) return;
		plan[at] = { ...plan[at], message };
		plan = [...plan];
	},

	/** For tests and previews: a planned rebase, without the backend. */
	seed(next: { upstream: string; todo: RebaseTodo; plan?: RebaseEdit[]; preview: RebasePreview }) {
		upstream = next.upstream;
		todo = next.todo;
		plan = next.plan ?? next.todo.rows.map((row) => ({ id: row.id, action: 'pick' as const }));
		preview = next.preview;
		focused = next.todo.rows[0]?.id ?? null;
		error = null;
		loading = false;
	},

	/** The message typed for a commit's reword, or empty. */
	messageOf(id: string): string {
		return plan[positionOf(id)]?.message ?? '';
	},

	/**
	 * Move a row by `by` places. Works from the keyboard as well as by drag,
	 * because drag alone is untestable headlessly and unusable for some people.
	 */
	async move(id: string, by: number): Promise<void> {
		const from = positionOf(id);
		if (from === -1) return;
		const to = from + by;
		if (to < 0 || to >= plan.length) return;

		const next = [...plan];
		const [moved] = next.splice(from, 1);
		next.splice(to, 0, moved);
		plan = next;
		focused = id;
		await this.recompute();
	},

	/** The token of the rebase running right now, or null. */
	get token(): number | null {
		return token;
	},
	get running(): boolean {
		return token !== null;
	},
	get progress(): RebaseProgress | null {
		return progress;
	},
	get outcome(): 'ran' | 'stopped' | 'failed' | null {
		return outcome;
	},
	get runError(): string | null {
		return runError;
	},
	get busy(): boolean {
		return busy;
	},
	/**
	 * True when git is part-way through and waiting for a person.
	 *
	 * Read from `progress` rather than from `outcome`, so that a rebase left
	 * unfinished by a previous session — or started from the command line — is
	 * recognised on arrival rather than only after this screen ran one.
	 */
	get stopped(): boolean {
		return token === null && progress !== null;
	},

	/**
	 * Start the plan. Resolves when it has *started*, not when it has finished.
	 *
	 * The confirmation belongs to the screen, not here: what has to be said
	 * depends on the branch and the count, and a store that asked as well would
	 * ask twice.
	 */
	async run(): Promise<boolean> {
		if (token !== null || plan.length === 0) return false;

		outcome = null;
		runError = null;
		progress = null;

		// What this run is worth, for the delight layer (FEAT-072). Counted here
		// because it is the only moment both facts are knowable: how many
		// commits were planned, and that this is a fresh run rather than a
		// continuation of one that stopped.
		planned = plan.length;
		stops = 0;

		try {
			token = await api.rebaseRun(plan);
			return true;
		} catch (e) {
			token = null;
			outcome = 'failed';
			runError = String(e);
			return false;
		}
	},

	/**
	 * Listen for the rebase's progress.
	 *
	 * Attached by the layout before anything can emit, the same as the clone's,
	 * so the first step of a fast rebase is never missed.
	 */
	async attach(): Promise<() => void> {
		const offProgress: UnlistenFn = await listen<RebaseProgressEvent>(
			REBASE_PROGRESS_EVENT,
			(event) => {
				// A step from a rebase that is no longer the one running belongs
				// to a rebase that has already been reported.
				if (event.payload.token !== token) return;
				progress = {
					step: event.payload.step,
					total: event.payload.total,
					branch: event.payload.branch,
					original: event.payload.original
				};
			}
		);

		const offDone: UnlistenFn = await listen<RebaseDoneEvent>(REBASE_DONE_EVENT, (event) => {
			if (event.payload.token !== token) return;
			token = null;

			outcome = event.payload.ok ? 'ran' : event.payload.stopped ? 'stopped' : 'failed';
			runError = event.payload.error;

			// A rebase that stopped will be continued; each stop is one fight
			// with a conflict, and finishing after any of them is what Rebase
			// Survivor is for.
			if (outcome === 'stopped') stops += 1;
			else if (outcome === 'ran') rebaseFinished(planned, stops, true);

			// A rebase that finished has no state left to read, and one that
			// stopped has the position it stopped at. Asked either way, because
			// the last progress event is a moment old and this is what the
			// hand-off to Conflicts is decided on.
			void this.refreshProgress();
			void repo.refresh();
		});

		return () => {
			offProgress();
			offDone();
		};
	},

	/** Ask the repository where a rebase stands, if one is standing anywhere. */
	async refreshProgress(): Promise<void> {
		try {
			progress = await api.rebaseProgress();
		} catch {
			// No repository open, or one that cannot be read. Neither is worth
			// putting an error on a screen about a rebase nobody started.
			progress = null;
		}
	},

	/** Carry on with a rebase that stopped, once its conflicts are resolved. */
	continue(): Promise<boolean> {
		return this.control(() => api.rebaseContinue());
	},

	/** Drop the commit it stopped on and carry on with the rest. */
	skip(): Promise<boolean> {
		return this.control(() => api.rebaseSkip());
	},

	/** Unwind the rebase and put the branch back where it started. */
	abort(): Promise<boolean> {
		return this.control(() => api.rebaseAbort());
	},

	/**
	 * Run one of the three controls, then re-read where that left things.
	 *
	 * They share a shape because they share a hazard: each one either finishes
	 * the rebase or leaves it stopped somewhere else, and the screen cannot tell
	 * which from the call's own result.
	 */
	async control(operation: () => Promise<void>): Promise<boolean> {
		if (busy || token !== null) return false;
		busy = true;
		runError = null;

		try {
			await operation();
			return true;
		} catch (e) {
			runError = String(e);
			return false;
		} finally {
			busy = false;
			await this.refreshProgress();
			outcome = progress === null ? 'ran' : 'stopped';
			await repo.refresh();
		}
	},

	focus(id: string | null): void {
		focused = id;
	},

	/** Put the plan back to the list git would have opened. */
	async reset(): Promise<void> {
		if (!todo) return;
		plan = todo.rows.map((row) => ({ id: row.id, action: 'pick' as const }));
		await this.recompute();
	},

	/** Recompute the preview. Called after every change; the last one wins. */
	async recompute(): Promise<void> {
		const current = ++previewSeq;
		try {
			const next = await api.rebasePreview(plan);
			if (current !== previewSeq) return;
			preview = next;
		} catch (e) {
			if (current === previewSeq) {
				error = String(e);
				preview = null;
			}
		}
	},

	clear(): void {
		previewSeq += 1;
		todo = null;
		plan = [];
		preview = null;
		upstream = '';
		loading = false;
		error = null;
		focused = null;
		// Not `token`: a rebase that is running belongs to the repository, not
		// to this screen's plan, and forgetting it here would orphan its events.
		progress = null;
		outcome = null;
		runError = null;
		busy = false;
	}
};

/**
 * What each action does, in one line, for the chip's title.
 *
 * Exported because the component and its tests both need it, and a label that
 * only exists inside markup cannot be tested at all.
 */
export const ACTION_MEANINGS: Record<RebaseAction, string> = {
	pick: 'Keep this commit as it is.',
	squash: 'Fold this commit into the one above it.',
	reword: 'Keep the change; git asks for a new message when the rebase runs.',
	drop: 'Leave this commit out of the result entirely.'
};

/** The actions in the order the chips offer them. */
export const ACTIONS: RebaseAction[] = ['pick', 'squash', 'reword', 'drop'];
