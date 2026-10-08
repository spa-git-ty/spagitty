// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Words for agents' work (2.0): the levels, the meter, the marks in a file.
 *
 * Pure, so each phrase has a test. The words are the spec's: a level is named
 * for what the person does, and the sentence says where the agent stops.
 */

import type {
	Assignment,
	Finding,
	Job,
	Level,
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
		label: 'Suggest',
		short: 'Suggest',
		review: 'Runs to the end. Nothing is applied until you accept it.',
		merge: 'Proposes every conflict and runs the checks. Nothing is applied until you accept it.'
	},
	{
		id: 'stepByStep',
		label: 'Step by step',
		short: 'Step',
		review: 'Stops after the plan and each file.',
		merge: 'Stops at each conflict and at the checks.'
	},
	{
		id: 'signOff',
		label: 'Sign off',
		short: 'Sign off',
		review: 'Drafts as it goes and stops before the review is sent.',
		merge: 'Resolves as it goes and stops before the merge lands.'
	},
	{
		id: 'unattended',
		label: 'Unattended',
		short: 'Unattended',
		review: 'Sends the review itself, as Comment unless this repository allows verdicts.',
		merge: 'Lands the merge itself, only when every conflict is sure and the checks pass.'
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

/** `1 / 4 · 1:20`: units done of planned, and time elapsed. */
export function meter(a: Assignment, now: number): string {
	const total = planned(a);
	const end = a.endedAt ?? now;
	const time = clock(Math.max(0, end - a.startedAt));
	return total > 0 ? `${unitsDone(a)} / ${total} · ${time}` : time;
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
	const total = planned(a);
	return total > 0 ? Math.min(1, unitsDone(a) / total) : 0;
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
