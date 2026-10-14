// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Recipes screen's state (FEAT-133).
 *
 * # A draft, and what is saved
 *
 * The stack on screen is a draft. Every edit to it is checked by the
 * validator in the backend — the same function that is asked again at *Run* —
 * and what it found is shown under the block it is about. *Save* writes the
 * draft over the recipe it was opened from; until then the saved one is as it
 * was.
 *
 * # The agent's help is a switch
 *
 * The author's note of 2026-10-13. On, the person writes a sentence and the
 * agent proposes the blocks, which arrive in the draft to be read and edited:
 * nothing it proposes runs. Off, the sentence, the note and *agent writes*
 * are disabled, and nothing in this store asks an agent for anything.
 *
 * # A run
 *
 * Spagitty runs the steps itself, on a thread in the backend, and sends the
 * whole run whenever it changes and each line of output as it is printed.
 * The run shown here is what was last sent; the lines that arrived since are
 * kept beside it until the next whole run takes their place.
 *
 * While a step is running the run holds the activity lanes (FEAT-121), as a
 * pull from the toolbar does: the toolbar names it, and a fetch or a stash
 * that would collide waits, with the reason. While the run waits for the
 * person the lanes are free, because what it waits for may be theirs to do.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { activity, type Step as Held } from '$lib/activity/store.svelte';
import * as agentsApi from '$lib/agents/api';
import { consentSlug } from '$lib/agents/levels';
import { agents } from '$lib/agents/store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import { describe, notice } from '$lib/ui/notice.svelte';
import * as api from './api';
import { fresh } from './catalogue';
import { added, blank, moved, nextName, removed, replaced, same, withoutAgent } from './model';
import type { BlockId, Check, Choice, Level, Line, Recipe, RunView, Step } from './types';

/** How long after an edit the stack is checked: a word typed is one check. */
export const CHECK_DELAY = 120;
/** How many edits can be taken back. */
const HISTORY = 50;

/** An agent the recipe can be helped by. */
export interface Helper {
	id: string;
	name: string;
	/** A model reached over an API, and what it is. */
	detail: string | null;
	/** The provider this repository has not yet agreed to send anything to. */
	needsConsent: { slug: string; label: string } | null;
}

let saved = $state<Recipe[]>([]);
let guideVersion = $state(0);
let draft = $state<Recipe | null>(null);
/** The name the draft was saved under. Null for one never saved. */
let savedAs = $state<string | null>(null);
let selected = $state(-1);
let checked = $state<Check | null>(null);
let loaded = $state(false);
/** Why the recipes could not be read. Stays until they can be. */
let loadError = $state<string | null>(null);
/** Why the stack could not be checked. Goes with the next check that can. */
let checkError = $state<string | null>(null);
let sentence = $state('');
let proposing = $state(false);
/** What the agent said about the blocks it proposed. */
let said = $state<string | null>(null);
let run = $state<RunView | null>(null);
/** Lines printed since the run was last sent whole. */
let fresher = $state<Line[]>([]);
let starting = $state(false);
/** A choice is being taken. */
let answering = $state(false);
/** Why a choice could not be taken yet. */
let refused = $state<string | null>(null);

let history: Recipe[] = [];
let checkSeq = 0;
let timer: ReturnType<typeof setTimeout> | null = null;
let unlisten: UnlistenFn[] = [];
let listening = false;
let repoPath: string | null = null;
/** The lanes, while a step is running. */
let held: Held | null = null;

/** Hold the lanes while a step of `view` is running, and let them go otherwise. */
function hold(view: RunView | null): void {
	if (!view || !view.live || !view.working || view.run.outcome !== null) {
		held?.end();
		held = null;
		return;
	}
	const what = `Running ${view.run.recipe.name} · step ${Math.min(view.run.at + 1, view.run.steps.length)} of ${view.run.steps.length}`;
	if (held) held.say(what);
	// Null when something else has a lane: the backend's own lock is what
	// keeps two gits apart, and this is only what the toolbar says.
	else held = activity.begin('other', what, ['repository', 'network']);
}

function clone<T>(value: T): T {
	return JSON.parse(JSON.stringify(value)) as T;
}

async function checkNow(): Promise<void> {
	if (timer) clearTimeout(timer);
	timer = null;
	const asked = draft;
	if (!asked) {
		checked = null;
		return;
	}
	const mine = ++checkSeq;
	try {
		const answer = await api.check(asked);
		if (mine !== checkSeq) return;
		checked = answer;
		checkError = null;
	} catch (cause) {
		if (mine === checkSeq) checkError = describe(cause);
	}
}

function scheduleCheck(): void {
	if (timer) clearTimeout(timer);
	timer = setTimeout(() => void checkNow(), CHECK_DELAY);
}

/** Take `next` as the draft, remembering the one before for *undo*. */
function edit(next: Recipe): void {
	if (!draft || same(next, draft)) return;
	history.push(draft);
	if (history.length > HISTORY) history.shift();
	draft = next;
	selected = Math.min(selected, next.steps.length - 1);
	scheduleCheck();
}

function show(recipe: Recipe, name: string | null): void {
	draft = clone(recipe);
	savedAs = name;
	history = [];
	selected = -1;
	sentence = '';
	said = null;
	void checkNow();
}

function take(view: RunView | null): void {
	if (view && repoPath && view.repo !== repoPath) return;
	run = view;
	fresher = [];
	refused = null;
	hold(view);
}

export const recipes = {
	get saved(): Recipe[] {
		return saved;
	},
	get guideVersion(): number {
		return guideVersion;
	},
	get draft(): Recipe | null {
		return draft;
	},
	get savedAs(): string | null {
		return savedAs;
	},
	/** The step the keys are on, or -1. */
	get selected(): number {
		return selected;
	},
	get check(): Check | null {
		return checked;
	},
	get loaded(): boolean {
		return loaded;
	},
	get error(): string | null {
		return loadError ?? checkError;
	},
	get sentence(): string {
		return sentence;
	},
	get proposing(): boolean {
		return proposing;
	},
	get said(): string | null {
		return said;
	},
	get run(): RunView | null {
		return run;
	},
	/** The run's output: what was sent with it, and what was printed since. */
	get lines(): Line[] {
		return run ? [...run.run.lines, ...fresher] : [];
	},
	get starting(): boolean {
		return starting;
	},
	get answering(): boolean {
		return answering;
	},
	get refused(): string | null {
		return refused;
	},

	/** True when the draft is not what is saved under its name. */
	get dirty(): boolean {
		if (!draft) return false;
		const kept = saved.find((recipe) => recipe.name === savedAs) ?? null;
		return !same(draft, kept);
	},

	/** True when there is an edit to take back. */
	get canUndo(): boolean {
		void draft;
		return history.length > 0;
	},

	/**
	 * The agents the recipe can be helped by, from Settings › Agents: the
	 * command-line agents that run and the models, less any this repository
	 * does not allow.
	 */
	get helpers(): Helper[] {
		const snapshot = agents.snapshot;
		if (!snapshot) return [];
		const allowed = (id: string) => snapshot.rules?.agents?.includes(id) ?? true;
		const agreed = snapshot.rules?.consent ?? [];
		return [
			...snapshot.local
				.filter((local) => local.availability.state === 'available' && allowed(local.id))
				.map((local) => ({ id: local.id, name: local.name, detail: null, needsConsent: null })),
			...snapshot.remote
				.filter((remote) => allowed(remote.id))
				.map((remote) => {
					const slug = consentSlug(remote);
					return {
						id: remote.id,
						name: remote.name,
						detail: remote.model,
						needsConsent:
							remote.local || agreed.includes(slug) ? null : { slug, label: remote.providerLabel }
					};
				})
		];
	},

	/** Read this repository's recipes, and the run on it if there is one. */
	async load(repo: string | null): Promise<void> {
		repoPath = repo;
		await this.listen();
		try {
			const [listed, current] = await Promise.all([api.list(), api.current()]);
			saved = listed.recipes;
			guideVersion = listed.guideVersion;
			// A run from before is shown only while it is going. One that is
			// over is in Reflog's restore points, not in the way of the builder.
			take(current && current.live && current.run.outcome === null ? current : null);
			loadError = null;
		} catch (cause) {
			saved = [];
			loadError = describe(cause);
		}
		loaded = true;
		if (saved.length > 0) show(saved[0], saved[0].name);
		else show(blank(nextName([])), null);
	},

	async listen(): Promise<void> {
		if (listening) return;
		listening = true;
		try {
			unlisten = [
				await listen<RunView>(api.RUN_EVENT, (event) => {
					if (run && event.payload.run.id !== run.run.id && run.run.outcome === null) return;
					if (!run && !starting) return;
					take(event.payload);
				}),
				await listen<{ repo: string; id: string; line: Line }>(api.LINE_EVENT, (event) => {
					if (run && event.payload.id === run.run.id) fresher = [...fresher, event.payload.line];
				})
			];
		} catch {
			// Not in the application: nothing will run.
		}
	},

	/** Open a saved recipe in the builder. */
	open(name: string): void {
		const recipe = saved.find((each) => each.name === name);
		if (recipe) show(recipe, name);
	},

	/** A new, empty recipe. */
	create(): void {
		show(blank(nextName(saved.map((recipe) => recipe.name))), null);
	},

	select(index: number): void {
		if (!draft) return;
		selected = Math.max(-1, Math.min(index, draft.steps.length - 1));
	},

	/** Add a block to the end of the stack, with what can be chosen chosen. */
	add(block: BlockId): void {
		if (!draft) return;
		edit(added(draft, fresh(block, checked?.world ?? null)));
		selected = draft.steps.length - 1;
	},

	remove(index: number): void {
		if (!draft || index < 0 || index >= draft.steps.length) return;
		edit(removed(draft, index));
	},

	/** Move a step up or down, and the keys with it. */
	move(index: number, by: -1 | 1): void {
		if (!draft) return;
		const next = moved(draft, index, by);
		if (next === draft) return;
		edit(next);
		if (selected === index) selected = index + by;
	},

	set(index: number, step: Step): void {
		if (draft) edit(replaced(draft, index, step));
	},

	rename(name: string): void {
		if (draft) edit({ ...draft, name });
	},

	setLevel(level: Level): void {
		if (draft) edit({ ...draft, level });
	},

	setNote(note: string): void {
		if (draft) edit({ ...draft, note });
	},

	/** The branch the recipe is made for: the one checked out, or any. */
	setBranch(branch: string): void {
		if (draft) edit({ ...draft, branch });
	},

	setSentence(text: string): void {
		sentence = text;
	},

	/**
	 * Switch the agent's help. Off, every message left to the agent becomes
	 * one to type, so the stack is still one that can run.
	 */
	setAgentHelp(on: boolean): void {
		if (!draft) return;
		if (!on) {
			edit(withoutAgent(draft));
			said = null;
			return;
		}
		const first = this.helpers.find((helper) => !helper.needsConsent)?.id ?? null;
		edit({ ...draft, agentHelp: true, agent: draft.agent ?? first });
	},

	/**
	 * Choose the agent. A model whose provider this repository has not agreed
	 * to send anything to is asked about first, once, and not chosen if the
	 * answer is no.
	 */
	async setAgent(id: string | null): Promise<void> {
		if (!draft) return;
		const helper = this.helpers.find((each) => each.id === id);
		if (helper?.needsConsent && repoPath) {
			const agreed = await dialog.confirm({
				title: `Send to ${helper.needsConsent.label}?`,
				body: `${helper.name} is reached over an API. Helping with a recipe sends it this repository's branch names, the changes a commit message is written from, and what a failed command printed. You can change this in Settings › Agents.`,
				confirmLabel: 'Agree'
			});
			if (!agreed) return;
			try {
				await agentsApi.consent(repoPath, helper.needsConsent.slug);
				await agents.load(repoPath);
			} catch (cause) {
				notice.failed('Could not record the consent', cause);
				return;
			}
		}
		if (draft) edit({ ...draft, agent: id });
	},

	/** Take the last edit back. */
	undo(): void {
		const before = history.pop();
		if (!before) return;
		draft = before;
		selected = Math.min(selected, before.steps.length - 1);
		scheduleCheck();
	},

	/** Check the draft now, without waiting for the pause after an edit. */
	checkNow,

	async save(): Promise<boolean> {
		if (!draft) return false;
		try {
			const name = draft.name.trim();
			saved = await api.save({ ...draft, name }, savedAs);
			draft = { ...draft, name };
			savedAs = name;
			notice.ok(`Saved ${name}`);
			return true;
		} catch (cause) {
			notice.failed('Could not save the recipe', cause);
			return false;
		}
	},

	/** Delete the saved recipe the draft was opened from. */
	async removeSaved(): Promise<void> {
		if (!savedAs) {
			this.create();
			return;
		}
		const gone = savedAs;
		const sure = await dialog.confirm({
			title: `Delete ${gone}?`,
			body: 'The recipe is removed from this machine. Runs it made, and their restore points, are kept.',
			confirmLabel: 'Delete',
			danger: true
		});
		if (!sure) return;
		try {
			saved = await api.remove(gone);
			if (saved.length > 0) show(saved[0], saved[0].name);
			else this.create();
		} catch (cause) {
			notice.failed('Could not delete the recipe', cause);
		}
	},

	/** The draft as the JSON it is exported in. */
	exported(): string {
		return draft ? JSON.stringify(draft, null, 2) : '';
	},

	/** Open a recipe from exported JSON, as a draft that is not yet saved. */
	async importFrom(text: string): Promise<boolean> {
		try {
			const recipe = await api.importFrom(text);
			const taken = saved.some((each) => each.name === recipe.name);
			show(taken ? { ...recipe, name: nextName(saved.map((each) => each.name)) } : recipe, null);
			return true;
		} catch (cause) {
			notice.failed('Could not read that recipe', cause);
			return false;
		}
	},

	/**
	 * Ask the agent for the blocks. What comes back replaces the stack as an
	 * edit like any other: it is validated, it can be changed, and Ctrl+Z
	 * brings back what was there.
	 */
	async propose(): Promise<void> {
		if (!draft || !draft.agentHelp || !draft.agent || proposing) return;
		const asked = sentence.trim();
		if (!asked) return;
		proposing = true;
		said = null;
		try {
			const proposed = await api.propose(draft, asked);
			said = proposed.said || null;
			if (draft && proposed.steps.length > 0) {
				edit({ ...draft, steps: proposed.steps });
				selected = -1;
			} else if (!proposed.said) {
				said = 'The agent proposed no blocks.';
			}
		} catch (cause) {
			notice.failed('The agent could not propose blocks', cause);
		} finally {
			proposing = false;
		}
	},

	/** Run the draft. It is checked again, against the repository as it is now. */
	async start(): Promise<boolean> {
		if (!draft || starting) return false;
		const busy = activity.blocked('repository', 'network');
		if (busy) {
			notice.failed('The recipe did not start', busy);
			return false;
		}
		starting = true;
		try {
			take(await api.start(draft));
			return true;
		} catch (cause) {
			notice.failed('The recipe did not start', cause);
			void checkNow();
			return false;
		} finally {
			starting = false;
		}
	},

	/** The person's choice at a stop. */
	async answer(choice: Choice): Promise<boolean> {
		if (!run || answering) return false;
		answering = true;
		refused = null;
		try {
			take(await api.answer(choice));
			return true;
		} catch (cause) {
			refused = describe(cause);
			return false;
		} finally {
			answering = false;
		}
	},

	async stop(): Promise<void> {
		try {
			await api.stop();
		} catch (cause) {
			notice.failed('Could not stop the run', cause);
		}
	},

	/** *Undo the whole run*, after asking. */
	async undoRun(): Promise<void> {
		if (!run) return;
		const sure = await dialog.confirm({
			title: 'Undo the whole run?',
			body: 'HEAD, the branches the recipe named, the working tree, the index and the stash list go back to what they were before the first step. What was pushed stays pushed.',
			confirmLabel: 'Undo the run',
			danger: true
		});
		if (!sure) return;
		try {
			take(await api.undo());
			notice.ok('The run was undone');
		} catch (cause) {
			notice.failed('Could not undo the run', cause);
		}
	},

	/** Leave a run that is over, back to the builder. */
	async closeRun(): Promise<void> {
		try {
			await api.close();
			take(null);
			void checkNow();
		} catch (cause) {
			notice.failed('The run is not over', cause);
		}
	},

	clearState(): void {
		if (timer) clearTimeout(timer);
		timer = null;
		for (const stop of unlisten) stop();
		unlisten = [];
		hold(null);
		listening = false;
		saved = [];
		guideVersion = 0;
		draft = null;
		savedAs = null;
		selected = -1;
		checked = null;
		loaded = false;
		loadError = null;
		checkError = null;
		sentence = '';
		proposing = false;
		said = null;
		run = null;
		fresher = [];
		starting = false;
		answering = false;
		refused = null;
		history = [];
		checkSeq = 0;
		repoPath = null;
	}
};
