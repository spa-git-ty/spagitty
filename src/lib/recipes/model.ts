// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A recipe being built (FEAT-133): the edits to its stack, and the words.
 *
 * Pure, so each edit and each phrase has a test. Nothing here decides
 * whether a stack can run: that is the validator's, in the core, and the
 * screen shows what it found.
 */

import type { Choice, Finding, Level, Recipe, Run, Step, Stop } from './types';

/** The limits the validator enforces, for the counters beside the fields. */
export const MAX_MESSAGE = 72;
export const MAX_NOTE = 140;
export const MAX_SENTENCE = 280;
export const MAX_STEPS = 30;
export const MAX_NAME = 40;

export function blank(name: string): Recipe {
	return { v: 1, name, branch: '', steps: [], level: 'stepByStep', agentHelp: false, agent: null, note: '' };
}

/** A name nothing saved has: `recipe-1`, `recipe-2`. */
export function nextName(taken: readonly string[]): string {
	for (let n = 1; ; n++) {
		const name = `recipe-${n}`;
		if (!taken.includes(name)) return name;
	}
}

/** `step` at the end of the stack. */
export function added(recipe: Recipe, step: Step): Recipe {
	return { ...recipe, steps: [...recipe.steps, step] };
}

export function removed(recipe: Recipe, index: number): Recipe {
	return { ...recipe, steps: recipe.steps.filter((_, at) => at !== index) };
}

/** The step at `index` one place up or down. Unchanged at an end. */
export function moved(recipe: Recipe, index: number, by: -1 | 1): Recipe {
	const to = index + by;
	if (index < 0 || index >= recipe.steps.length || to < 0 || to >= recipe.steps.length) return recipe;
	const steps = [...recipe.steps];
	[steps[index], steps[to]] = [steps[to], steps[index]];
	return { ...recipe, steps };
}

export function replaced(recipe: Recipe, index: number, step: Step): Recipe {
	return { ...recipe, steps: recipe.steps.map((each, at) => (at === index ? step : each)) };
}

/** Whether two recipes are the same recipe: what *Save* is dimmed by. */
export function same(a: Recipe | null, b: Recipe | null): boolean {
	return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * A recipe with the agent's help switched off, as the author asked: every
 * message left to the agent becomes one to be typed, and nothing else of the
 * stack changes. The note is kept, unread, for when the help is on again.
 */
export function withoutAgent(recipe: Recipe): Recipe {
	const typed = (step: Step): Step => {
		switch (step.block) {
			case 'commit':
			case 'squash':
			case 'reword':
				return step.params.message.kind === 'agent'
					? ({ ...step, params: { ...step.params, message: { kind: 'text', text: '' } } } as Step)
					: step;
			case 'amend':
				return step.params.message?.kind === 'agent'
					? { ...step, params: { message: { kind: 'text', text: '' } } }
					: step;
			default:
				return step;
		}
	};
	return { ...recipe, agentHelp: false, steps: recipe.steps.map(typed) };
}

export interface LevelWords {
	id: Level;
	label: string;
	/** For the row of four, where the full names do not fit. */
	short: string;
	/** Where it stops: the line under the row. */
	says: string;
}

/** The four levels of 2.0, in the words a recipe means by them. */
export const LEVELS: LevelWords[] = [
	{
		id: 'suggest',
		label: 'Read-only',
		short: 'Read-only',
		says: 'Nothing runs. The commands are shown, and Copy as script is the way out.'
	},
	{ id: 'stepByStep', label: 'Ask every step', short: 'Every step', says: 'Stops before every step.' },
	{
		id: 'signOff',
		label: 'Ask before pushing',
		short: 'Before pushing',
		says: 'Runs, and stops before Push and Delete branch.'
	},
	{
		id: 'unattended',
		label: 'Full access',
		short: 'Full access',
		says: 'Runs through. A conflict, a failure or a rejected push still stops it.'
	}
];

export function levelWords(level: Level): LevelWords {
	return LEVELS.find((words) => words.id === level) ?? LEVELS[1];
}

/** What the validator found about the step at `index`, or about the recipe. */
export function findingsAt(findings: readonly Finding[], index: number | null): Finding[] {
	return findings.filter((finding) => finding.step === index);
}

/** What *Run* says: how many steps, or how many problems stand in the way. */
export function runLabel(steps: number, errors: number): string {
	if (errors > 0) return `${errors} ${errors === 1 ? 'problem' : 'problems'}`;
	return `Run · ${steps} ${steps === 1 ? 'step' : 'steps'}`;
}

/** `0:02`, `1:20`: how long a step took. */
export function took(ms: number | null): string {
	if (ms === null) return '';
	const seconds = Math.floor(ms / 1000);
	return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
}

/** The run in a few words, for the pill beside its name. */
export function runStatus(run: Run, working: boolean): string {
	const of = `step ${Math.min(run.at + 1, run.steps.length)} of ${run.steps.length}`;
	if (run.outcome === 'done') return 'Done';
	if (run.outcome === 'undone') return 'Undone';
	if (run.outcome === 'ended') return `Ended at ${of}`;
	if (run.stop) return `Waiting for you · ${of}`;
	return working ? `Running · ${of}` : `Starting · ${of}`;
}

/** How much of the run is behind it, 0 to 1. */
export function runShare(run: Run): number {
	if (run.steps.length === 0) return 0;
	const behind = run.steps.filter((step) => step.state === 'done' || step.state === 'skipped').length;
	return behind / run.steps.length;
}

/** A choice at a stop, as its button says it. */
export function choiceLabel(choice: Choice, stop: Stop): string {
	switch (choice) {
		case 'continue':
			if (stop.kind === 'approve') return `Run step ${stop.step + 1}`;
			if (stop.kind === 'changed') return 'Go on anyway';
			if (stop.kind === 'conflict') return stop.inMerger ? 'It is merged: go on' : 'It is resolved: go on';
			return 'Go on';
		case 'retry':
			return 'Run it again';
		case 'skip':
			return 'Skip this step';
		case 'openMerger':
			return 'Open in Merger';
		case 'openConflicts':
			return 'Open in Conflicts';
		case 'abort':
			return stop.step === 0
				? `Abort the ${stop.operation ?? 'operation'}`
				: `Abort the ${stop.operation ?? 'operation'}, keep ${stop.step === 1 ? 'step 1' : `steps 1–${stop.step}`}`;
		case 'end':
			return 'End the run';
	}
}

/** The choice a stop's filled button is: the one that goes on. */
export function mainChoice(stop: Stop): Choice | null {
	const order: Choice[] = stop.inMerger
		? ['continue']
		: stop.kind === 'conflict'
			? ['openMerger', 'openConflicts']
			: ['continue', 'retry'];
	return order.find((choice) => stop.choices.includes(choice)) ?? null;
}
