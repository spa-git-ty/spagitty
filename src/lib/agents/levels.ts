// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Words for agents' work (2.0): the levels, the meter, the marks in a file.
 *
 * Pure, so each phrase has a test. A permission is named for what the agent
 * may do, in the words other agent tools use (TASK-060): read-only, ask, full
 * access. The stored values (`suggest`, `stepByStep`, `signOff`,
 * `unattended`) are older than the names and stay, so saved settings keep
 * working; only what is shown changes.
 */

import type {
	Assignment,
	Finding,
	Job,
	Level,
	PlanItem,
	Proposal,
	RemoteAgent,
	Step
} from './types';

export interface LevelWords {
	id: Level;
	label: string;
	/** For a chip row where four full names do not fit. */
	short: string;
	/** Where it stops, per job: the line under the level picker. */
	review: string;
	merge: string;
}

export const LEVELS: LevelWords[] = [
	{
		id: 'suggest',
		label: 'Read-only',
		short: 'Read-only',
		review: 'Reads and proposes. Nothing is applied until you accept it.',
		merge: 'Proposes every conflict and runs the checks. Nothing is applied until you accept it.'
	},
	{
		id: 'stepByStep',
		label: 'Ask every step',
		short: 'Ask every step',
		review: 'Stops for you after the plan and after each file.',
		merge: 'Stops for you at each conflict and at the checks.'
	},
	{
		id: 'signOff',
		label: 'Ask before sending or merging',
		short: 'Ask at the end',
		review: 'Works through everything and stops before the review is sent.',
		merge: 'Resolves everything and stops before the merge is made.'
	},
	{
		id: 'unattended',
		label: 'Full access',
		short: 'Full access',
		review: 'Sends the review itself, as a comment unless this repository allows verdicts.',
		merge: 'Makes the merge itself, only when every conflict is sure and the checks pass.'
	}
];

const ORDER: Level[] = LEVELS.map((level) => level.id);

export function levelWords(level: Level): LevelWords {
	return LEVELS.find((words) => words.id === level) ?? LEVELS[1];
}

export function sentenceFor(level: Level, job: Job): string {
	return levelWords(level)[job];
}

/** Is `level` above the repository's highest? */
export function above(level: Level, highest: Level): boolean {
	return ORDER.indexOf(level) > ORDER.indexOf(highest);
}

export function capped(level: Level, highest: Level): Level {
	return above(level, highest) ? highest : level;
}

/** The slug a repository's consent is kept under, as the backend names it. */
export function consentSlug(agent: Pick<RemoteAgent, 'provider'>): string {
	switch (agent.provider) {
		case 'anthropic':
			return 'anthropic';
		case 'openAi':
			return 'openai';
		case 'google':
			return 'google';
		case 'compatible':
			return 'compatible';
	}
}

export function isLive(a: Assignment): boolean {
	return a.state !== 'done' && a.state !== 'stopped' && a.state !== 'failed';
}

/** Files or conflicts: the meter's unit. */
function isUnit(step: Step): boolean {
	return step.kind.kind === 'file' || step.kind.kind === 'conflict';
}

/** How many units are planned: the plan's files, or the conflicts. */
export function planned(a: Assignment): number {
	const rest = a.job === 'review' ? 3 : 2 + (a.lands ? 1 : 0);
	return Math.max(0, a.planned - rest);
}

export function unitsDone(a: Assignment): number {
	return a.steps.filter((s) => isUnit(s) && s.state === 'done').length;
}

/** Longer than this with nothing said, and the agent is quiet, not working. */
export const QUIET_AFTER = 180;

/** The step the agent is on: the one running or waiting, else the last. */
export function currentStep(a: Assignment): Step | null {
	const steps = a.steps.filter((s) => s.kind.kind !== 'why');
	return [...steps].reverse().find((s) => s.state === 'running' || s.state === 'waiting') ?? steps.at(-1) ?? null;
}

export interface Progress {
	done: number;
	/** Null until the plan says how many there will be. */
	total: number | null;
	unit: 'files' | 'conflicts' | 'steps';
}

/**
 * How far through the run it is: the files or conflicts when the plan has
 * said how many, and the steps when it has not (FEAT-119). Every run has the
 * steps; only some have units, and only after the plan.
 */
export function progressOf(a: Assignment): Progress | null {
	const units = planned(a);
	if (units > 0) return { done: unitsDone(a), total: units, unit: a.job === 'review' ? 'files' : 'conflicts' };
	const steps = a.steps.filter((s) => s.kind.kind !== 'why');
	if (steps.length === 0) return null;
	return {
		done: steps.filter((s) => s.state === 'done').length,
		total: a.planned > 0 ? Math.max(a.planned, steps.length) : null,
		unit: 'steps'
	};
}

/** Seconds the agent has said nothing for, once that is long enough to say so; else null. */
export function quietFor(a: Assignment, now: number): number | null {
	if (!isLive(a) || a.state !== 'working' || a.quietSince === null) return null;
	const quiet = Math.max(0, now - a.quietSince);
	return quiet >= QUIET_AFTER ? quiet : null;
}

/**
 * At work right now: started, not stopped for the person or paused, and heard
 * from lately (FEAT-124). Only this moves on screen, so a run that has gone
 * quiet stops moving when it stops talking.
 */
export function isWorking(a: Assignment, now: number): boolean {
	return (a.state === 'starting' || a.state === 'working') && quietFor(a, now) === null;
}

/** `3 min`, or `1 h 5 min`. */
export function quietWords(seconds: number): string {
	const minutes = Math.max(1, Math.floor(seconds / 60));
	if (minutes < 60) return `${minutes} min`;
	return `${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}

/** What the agent is doing now, in words: the step, and how long it has taken. */
export function nowLine(a: Assignment, now: number): string | null {
	if (!isLive(a)) return null;
	const step = currentStep(a);
	if (!step) return null;
	return `${step.label} · ${stepTime(step, now)}`;
}

/** `1 / 4 · 1:20` for files or conflicts, `2 / 5 steps · 1:20` before there are any. */
export function meter(a: Assignment, now: number): string {
	const end = a.endedAt ?? now;
	const time = clock(Math.max(0, end - a.startedAt));
	const progress = progressOf(a);
	if (!progress || progress.total === null) return time;
	return progress.unit === 'steps'
		? `${progress.done} / ${progress.total} steps · ${time}`
		: `${progress.done} / ${progress.total} · ${time}`;
}

/** `0:12`, `1:20`, `1:02:05`. */
export function clock(seconds: number): string {
	const s = Math.floor(seconds % 60);
	const m = Math.floor(seconds / 60) % 60;
	const h = Math.floor(seconds / 3600);
	const pad = (n: number) => String(n).padStart(2, '0');
	return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** The share of the meter's bar, 0 to 1. */
export function share(a: Assignment): number {
	const progress = progressOf(a);
	if (!progress || !progress.total) return 0;
	return Math.min(1, progress.done / progress.total);
}

/** The step's duration, for its row. */
export function stepTime(step: Step, now: number): string {
	return clock(Math.max(0, (step.endedAt ?? now) - step.startedAt));
}

/** The file the agent is on right now, if it is on one. */
export function readingNow(a: Assignment): string | null {
	if (!isLive(a)) return null;
	const last = a.steps.at(-1);
	if (!last || (last.state !== 'running' && last.state !== 'waiting')) return null;
	return last.kind.kind === 'file' || last.kind.kind === 'conflict' ? last.kind.path : null;
}

/** Files the agent has finished reading — kept apart from *Viewed*. */
export function agentRead(a: Assignment): Set<string> {
	const read = new Set<string>();
	for (const step of a.steps) {
		if (step.kind.kind === 'file' && step.state === 'done') read.add(step.kind.path);
	}
	return read;
}

/** The agent's findings still waiting for the person. */
export function openFindings(a: Assignment): (Proposal & { body: { kind: 'comment' } & Finding })[] {
	return a.proposals.filter(
		(p): p is Proposal & { body: { kind: 'comment' } & Finding } =>
			p.body.kind === 'comment' && p.state === 'proposed'
	);
}

/** Open findings per path. */
export function findingsByPath(a: Assignment): Map<string, number> {
	const counts = new Map<string, number>();
	for (const finding of openFindings(a)) {
		counts.set(finding.body.path, (counts.get(finding.body.path) ?? 0) + 1);
	}
	return counts;
}

/** The proposals of the step that waits at a gate, if one does. */
export function atGate(a: Assignment): { step: Step; proposals: Proposal[] } | null {
	const step = a.steps.find((s) => s.state === 'waiting');
	if (!step || a.state !== 'waiting') return null;
	return { step, proposals: a.proposals.filter((p) => p.step === step.index && p.state === 'proposed') };
}

/** What the agent stopped to ask, for the top of its card (FEAT-124). */
export interface Ask {
	/** The step it stopped on; null when it stopped between steps. */
	step: Step | null;
	/** What it proposed there and nobody has decided. */
	proposals: Proposal[];
	/** One line: what is asked. */
	summary: string;
	/** The name of what opens under the line, or null when the line is all of it. */
	more: string | null;
}

/** The plan the agent last proposed, if it proposed one. */
export function planOf(a: Assignment): { files: PlanItem[]; lookFor: string } | null {
	const plan = [...a.proposals].reverse().find((p) => p.body.kind === 'plan');
	return plan && plan.body.kind === 'plan' ? plan.body : null;
}

/**
 * What waits for the person, in one line, and whether there is more to open.
 * The line is short enough to sit beside its buttons; the plan's files, a
 * failed check's output and the findings themselves are the more.
 */
export function askOf(a: Assignment, names: { a: string; b: string } = { a: 'A', b: 'B' }): Ask | null {
	if (a.state !== 'waiting') return null;
	const at = atGate(a);
	if (!at) return a.reason ? { step: null, proposals: [], summary: a.reason, more: null } : null;
	const { step, proposals } = at;
	const ask = (summary: string, more: string | null = null): Ask => ({ step, proposals, summary, more });
	if (step.gate === 'plan') {
		const plan = planOf(a);
		return ask(plan?.lookFor || 'The reading plan', plan?.files.length ? 'The plan' : null);
	}
	if (step.gate === 'send') return ask('Ready for you to send.');
	if (step.gate === 'land') return ask('Ready for you to merge.');
	if (step.gate === 'checks') {
		const failed = step.checks.find((c) => !c.passed);
		return failed ? ask(`${failed.command} failed`, 'The output') : ask('The checks passed.');
	}
	if (proposals.length === 0) return ask('Nothing to decide here.');
	const only = proposals.length === 1 ? proposals[0].body : null;
	if (only?.kind === 'resolution') return ask(only.why || choiceWords(only.choice, names));
	if (only?.kind === 'verdict') return ask(step.label, 'The summary');
	const count = `${proposals.length} ${proposals.length === 1 ? 'finding' : 'findings'}`;
	const severities = (['high', 'medium', 'low'] as const)
		.map((severity) => ({
			severity,
			count: proposals.filter((p) => p.body.kind === 'comment' && p.body.severity === severity).length
		}))
		.filter((each) => each.count > 0)
		.map((each) => `${each.count} ${each.severity}`);
	return ask(
		severities.length ? `${count} · ${severities.join(', ')}` : count,
		proposals.length === 1 ? 'The finding' : 'The findings'
	);
}

/** Where a finding is, as a reader says it: `avatars.rs:16`. */
export function findingPlace(finding: Pick<Finding, 'path' | 'line'>): string {
	return `${finding.path.split('/').pop()}:${finding.line}`;
}

/** The badge on a finding: `Codex · proposed · high`. */
export function findingLabel(agent: string, finding: Finding): string {
	return `${agent} · proposed · ${finding.severity}`;
}

/** The status chip on a conflict card. */
export function choiceWords(choice: { mode: string }, names: { a: string; b: string }): string {
	switch (choice.mode) {
		case 'a':
			return `Take ${names.a}`;
		case 'b':
			return `Take ${names.b}`;
		case 'ab':
			return `Both, ${names.a} first`;
		case 'ba':
			return `Both, ${names.b} first`;
		case 'pick':
			return 'Pick lines';
		default:
			return 'Edit';
	}
}

/** What leaves the machine, said once per agent and per repository. */
export function leaves(agent: Pick<RemoteAgent, 'local' | 'providerLabel'>): string {
	return agent.local
		? 'Everything stays on this machine.'
		: `Code you assign it is sent to ${agent.providerLabel}. Each repository asks once.`;
}

/** `40,000` — tokens read the way people count them. */
export function grouped(value: number): string {
	return Math.round(value).toLocaleString('en-US');
}

/** `40k / run`. */
export function perRun(tokens: number | null): string {
	if (!tokens) return 'no limit';
	return tokens >= 1000 ? `${Math.round(tokens / 1000)}k / run` : `${tokens} / run`;
}

/** The small line under an inbox card or a Result card: `Codex · 3 of 11
 * files`, `Claude Code · waiting for you`. */
export function cardLine(a: Assignment, now: number = Math.floor(Date.now() / 1000)): string {
	const unit = a.job === 'review' ? 'files' : 'conflicts';
	let what: string;
	if (a.state === 'waiting') what = 'waiting for you';
	else if (a.state === 'done') what = 'finished';
	else if (a.state === 'failed') what = 'failed';
	else if (a.state === 'stopped') what = 'stopped';
	else if (a.state === 'paused') what = 'paused';
	else if (quietFor(a, now) !== null) what = `quiet for ${quietWords(quietFor(a, now)!)}`;
	else if (planned(a) > 0) what = `${unitsDone(a)} of ${planned(a)} ${unit}`;
	else if (a.state === 'working' && currentStep(a)) what = currentStep(a)!.label;
	else what = 'starting';
	return `${a.agent.name} · ${what}`;
}
