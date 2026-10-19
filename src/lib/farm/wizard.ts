// SPDX-License-Identifier: GPL-3.0-or-later

import { AUTONOMY_LEVELS } from './describe';
import type { Autonomy, Brief, FarmSettings, GoalSource, PastFarm, Setup } from './types';
import type { Issue } from '$lib/issues/types';

/**
 * Starting a farm, one step at a time (FEAT-141): Goal → Start from →
 * Crew & rules → Start.
 *
 * What each step shows, what Continue and Skip do, what the start button says
 * and what the planner will be given are all decided here, so the tests read
 * the wizard's behaviour without mounting it. Only a one-line goal is
 * required; everything else has an answer already, either remembered from the
 * last farm in this repository or the defaults.
 */

export const STEPS = ['Goal', 'Start from', 'Crew & rules', 'Start'] as const;

export type Step = 1 | 2 | 3 | 4;

/** How the tasks are made, on step 4. */
export type Way = 'plan' | 'perIssue' | 'empty';

export interface WizardState {
	step: Step;
	title: string;
	notes: string;
	issues: Issue[];
	briefs: Brief[];
	branch: string | null;
	/** Step 3 has been seen, so the crew and rules are the person's, not last time's. */
	crewSeen: boolean;
	way: Way;
}

export function initial(title = '', notes = ''): WizardState {
	return { step: 1, title, notes, issues: [], briefs: [], branch: null, crewSeen: false, way: 'plan' };
}

/** The defaults a repository with no farm before it starts from. */
export const DEFAULT_RULES: FarmSettings = {
	autonomy: 'semiAuto',
	maxParallel: 2,
	maxAttempts: 3,
	verification: [],
	workspace: 'worktree',
	conflicts: { by: 'ask' }
};

/** The rules a remembered setup gives the wizard. */
export function rulesOf(setup: Setup | null): FarmSettings {
	if (!setup) return { ...DEFAULT_RULES };
	return {
		autonomy: setup.autonomy,
		permissions: { ...setup.permissions },
		maxParallel: setup.maxParallel,
		maxAttempts: setup.maxAttempts,
		verification: [...setup.verification],
		supplemental: { ...setup.supplemental },
		workspace: setup.workspace ?? 'worktree',
		conflicts: setup.conflicts ?? { by: 'ask' }
	};
}

/** Can the bar's primary button go on from this step? Only the goal is required. */
export function canContinue(state: WizardState): boolean {
	return state.step !== 1 || state.title.trim().length > 0;
}

/** Continue, or Skip where a step is optional: both go to the next step. */
export function next(state: WizardState): WizardState {
	if (!canContinue(state) || state.step === 4) return state;
	const step = (state.step + 1) as Step;
	return { ...state, step, crewSeen: state.crewSeen || step >= 3 };
}

export function back(state: WizardState): WizardState {
	if (state.step === 1) return state;
	return { ...state, step: (state.step - 1) as Step };
}

/** Skip to start: straight to step 4 with the remembered crew and rules. */
export function skipToStart(state: WizardState): WizardState {
	if (!canContinue(state)) return state;
	return { ...state, step: 4 };
}

/** Whether a step has a Skip beside Continue. Only Start from is optional and skippable. */
export function skippable(step: Step): boolean {
	return step === 2;
}

/** The ways step 4 offers. One task per issue only when issues were picked. */
export function ways(state: WizardState): Way[] {
	return state.issues.length ? ['plan', 'perIssue', 'empty'] : ['plan', 'empty'];
}

/** The way that is chosen, falling back when one stops being offered. */
export function wayOf(state: WizardState): Way {
	return ways(state).includes(state.way) ? state.way : 'plan';
}

function plural(n: number, one: string, many = `${one}s`): string {
	return `${n} ${n === 1 ? one : many}`;
}

/** What the primary button on step 4 says: what it will do. */
export function startLabel(way: Way, planner: string | null, issues: number): string {
	switch (way) {
		case 'plan':
			return planner ? `Plan it with ${planner}` : 'Plan it';
		case 'perIssue':
			return `Make ${plural(issues, 'task')}`;
		case 'empty':
			return 'Open an empty plan';
	}
}

/** What each way's card says under its name. */
export function wayDetail(way: Way, issues: Issue[]): string {
	switch (way) {
		case 'plan':
			return 'Reads everything you gave it and the repository, then proposes tasks in waves.';
		case 'perIssue': {
			const numbers = issues.map((issue) => `#${issue.number}`);
			const list = numbers.length > 1 ? `${numbers.slice(0, -1).join(', ')} and ${numbers.at(-1)}` : numbers[0];
			return `${plural(issues.length, 'task')}, straight from ${list}. Edit them before they run.`;
		}
		case 'empty':
			return 'Start from an empty plan.';
	}
}

export const WAY_NAMES: Record<Way, string> = {
	plan: 'Plan it with an agent',
	perIssue: 'One task per issue',
	empty: 'I’ll write the tasks'
};

/** One chip of **The planner gets**: what goes into the planning prompt. */
export interface PlannerChip {
	label: string;
	tone: 'goal' | 'issue' | 'brief' | 'plain';
	mono: boolean;
}

/**
 * Exactly what goes into the planning prompt, in its order: the goal and its
 * notes, each issue, each brief, the branch, `AGENTS.md` when there is one,
 * and the map of the repository (`context::planning`).
 */
export function plannerGets(state: WizardState, hasPolicy: boolean): PlannerChip[] {
	const chips: PlannerChip[] = [
		{ label: state.notes.trim() ? 'Goal and notes' : 'The goal', tone: 'goal', mono: false }
	];
	for (const issue of state.issues) chips.push({ label: `#${issue.number}`, tone: 'issue', mono: true });
	for (const brief of state.briefs) {
		chips.push({ label: brief.path.split('/').pop() ?? brief.path, tone: 'brief', mono: true });
	}
	if (state.branch) chips.push({ label: state.branch, tone: 'plain', mono: true });
	if (hasPolicy) chips.push({ label: 'AGENTS.md', tone: 'plain', mono: true });
	chips.push({ label: 'a map of the repository', tone: 'plain', mono: false });
	return chips;
}

/** The goal's sources, as the farm is created with them. */
export function sourcesOf(state: WizardState): GoalSource[] {
	return [
		...state.issues.map(
			(issue): GoalSource => ({
				kind: 'issue',
				number: issue.number,
				title: issue.title,
				body: issue.body,
				labels: issue.labels.map((label) => label.name),
				url: issue.url
			})
		),
		...state.briefs.map((brief): GoalSource => ({ kind: 'brief', path: brief.path, text: brief.text })),
		...(state.branch ? [{ kind: 'branch', name: state.branch } as GoalSource] : [])
	];
}

/** `3 issues · 1 brief · feature/gitea`, or that nothing was picked. */
export function sourcesLine(state: WizardState): string | null {
	const parts = [
		state.issues.length ? plural(state.issues.length, 'issue') : '',
		state.briefs.length ? plural(state.briefs.length, 'brief') : '',
		state.branch ?? ''
	].filter(Boolean);
	return parts.length ? parts.join(' · ') : null;
}

const SHORT_LEVELS: Record<Autonomy, string> = {
	manual: 'Manual',
	assisted: 'Assisted',
	semiAuto: 'Semi-auto',
	auto: 'Auto',
	yolo: 'YOLO'
};

/** The five levels as the segmented control names them. */
export const LEVELS = AUTONOMY_LEVELS.map((level) => ({ id: level.id, label: SHORT_LEVELS[level.id] }));

/** `Semi-automatic · 3 at once · 2 checks`. */
export function rulesLine(rules: FarmSettings): string {
	const level = AUTONOMY_LEVELS.find((l) => l.id === (rules.autonomy ?? 'semiAuto'))?.label ?? '';
	const checks = (rules.verification ?? []).filter((c) => c.trim()).length;
	return [level, `${rules.maxParallel ?? 2} at once`, checks ? plural(checks, 'check') : 'no checks'].join(' · ');
}

/** One row of **Your farm**. */
export interface SoFarRow {
	label: string;
	value: string;
	/** Not yet the person's: drawn dimmed. */
	dim: boolean;
	/** Remembered from the last farm, and not yet looked at. */
	lastTime: boolean;
	badges: boolean;
}

/** **Your farm**, filling in as the person goes. */
export function soFar(
	state: WizardState,
	context: { crew: number; rules: FarmSettings; remembered: boolean; planner: string | null }
): SoFarRow[] {
	const sources = sourcesLine(state);
	const fromLastTime = context.remembered && !state.crewSeen;
	const way = wayOf(state);
	const tasks =
		state.step < 4
			? '—'
			: way === 'plan'
				? `Planned by ${context.planner ?? 'an agent'}`
				: way === 'perIssue'
					? `One per issue · ${state.issues.length}`
					: 'Written by you';
	return [
		{ label: 'Goal', value: state.title.trim() || 'One line is enough', dim: !state.title.trim(), lastTime: false, badges: false },
		{ label: 'Start from', value: sources ?? 'Nothing yet · optional', dim: !sources, lastTime: false, badges: false },
		{ label: 'Crew', value: plural(context.crew, 'agent'), dim: !state.crewSeen, lastTime: fromLastTime, badges: true },
		{ label: 'Rules', value: rulesLine(context.rules), dim: !state.crewSeen, lastTime: fromLastTime, badges: false },
		{ label: 'Tasks', value: tasks, dim: state.step < 4, lastTime: false, badges: false }
	];
}

/** `Stopped · 4 of 7 merged`, and the colour token it is said in. */
export function pastLine(past: PastFarm): { text: string; colour: string; action: string } {
	const progress = `${past.merged} of ${past.total} merged`;
	if (past.status === 'completed') {
		return { text: `Done · ${progress}`, colour: 'var(--ok)', action: 'Use as a starting point →' };
	}
	return { text: `Stopped · ${progress}`, colour: 'var(--warn)', action: 'Resume →' };
}

/** What *Use as a starting point* copies: the goal, its context and its rules. */
export function startingPoint(past: PastFarm): { title: string; notes: string; rules: FarmSettings } {
	return { title: past.goal.title, notes: past.goal.description, rules: rulesOf(past.setup) };
}

/** `From “GitLab support for pull requests”, Oct 7.` under step 3's question. */
export function rememberedLine(setup: Setup | null): string {
	if (!setup || !setup.fromTitle) return 'These are the defaults. Change anything, or go on.';
	const when = setup.fromMs
		? `, ${new Date(setup.fromMs).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}`
		: '';
	return `From “${setup.fromTitle}”${when}. Change anything, or go on.`;
}
