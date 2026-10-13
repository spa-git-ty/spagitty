// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Recipes for the Recipes tests (FEAT-133): a repository's world, a stand-in
 * for the validator's answer, and a run.
 *
 * The validator is in Rust and is tested there, rule by rule. What stands in
 * for it here answers with what a test told it to find, so the screen is
 * tested for showing findings, not for making them.
 *
 * Lives outside `src/lib` so it is not counted as first-party code by the
 * coverage floor.
 */

import type { AgentsSnapshot } from '$lib/agents/types';
import type { Check, Finding, Recipe, Run, RunView, Step, StepRun, Stop, World } from '$lib/recipes/types';

export const WORLD: World = {
	branches: ['feature/avatars', 'main', 'old'],
	remotes: ['origin', 'upstream'],
	current: 'feature/avatars',
	dirty: false,
	operation: null,
	upstream: true,
	unpushed: 2,
	rewritesPushed: [],
	protected: ['main'],
	checks: ['cargo test', 'bun test'],
	agents: ['claude'],
	highest: 'unattended'
};

export const SYNC: Recipe = {
	v: 1,
	name: 'sync-and-ship',
	branch: '',
	steps: [
		{ block: 'stash', params: { untracked: true } },
		{ block: 'pull', params: { remote: 'origin', mode: 'fastForwardOnly' } },
		{ block: 'merge', params: { branch: 'main', how: 'commit' } },
		{ block: 'pop', params: {} },
		{ block: 'commit', params: { all: true, message: { kind: 'text', text: 'Sync with main' } } },
		{ block: 'push', params: { remote: 'origin' } }
	],
	level: 'stepByStep',
	agentHelp: false,
	agent: null,
	note: ''
};

export const SQUASH: Recipe = {
	v: 1,
	name: 'squash-for-pr',
	branch: 'feature/avatars',
	steps: [{ block: 'squash', params: { count: 3, message: { kind: 'agent' } } }],
	level: 'signOff',
	agentHelp: true,
	agent: 'claude',
	note: 'Keep it short.'
};

/** A step's command, near enough to read in a test. */
export function commandOf(step: Step): string {
	switch (step.block) {
		case 'stash':
			return step.params.untracked ? 'git stash push -u' : 'git stash push';
		case 'pop':
			return 'git stash pop';
		case 'commit':
			return step.params.message.kind === 'text'
				? `git commit -m "${step.params.message.text}"`
				: 'git commit -m "<agent writes>"';
		case 'pull':
			return `git pull ${step.params.remote}`.trim();
		case 'push':
			return `git push --set-upstream ${step.params.remote} HEAD`;
		case 'merge':
			return `git merge ${step.params.branch}`.trim();
		case 'switch':
			return `git switch ${step.params.branch}`.trim();
		case 'check':
			return step.params.command;
		default:
			return `git ${step.block}`;
	}
}

/** What the backend's check answers for `recipe`, finding `findings`. */
export function checkOf(recipe: Recipe, findings: Finding[] = [], world: World = WORLD): Check {
	const level =
		['suggest', 'stepByStep', 'signOff', 'unattended'].indexOf(recipe.level) >
		['suggest', 'stepByStep', 'signOff', 'unattended'].indexOf(world.highest)
			? world.highest
			: recipe.level;
	return {
		findings,
		errors: findings.filter((finding) => finding.severity === 'error').length,
		labels: recipe.steps.map((step, index) => `Step ${index + 1}: ${step.block}`),
		commands: recipe.steps.map(commandOf),
		brief: recipe.agentHelp ? `# Spagitty recipes\n---\nrecipe: ${recipe.name}\n` : null,
		script: `#!/bin/sh\nset -e\n${recipe.steps.map(commandOf).join('\n')}\n`,
		level,
		world
	};
}

export function error(step: number | null, code: string, text: string): Finding {
	return { step, severity: 'error', code, text };
}

export function warning(step: number | null, code: string, text: string): Finding {
	return { step, severity: 'warning', code, text };
}

/** A run of `recipe` that has done `done` steps. */
export function runOf(recipe: Recipe, done: number, more: Partial<Run> = {}): Run {
	const steps: StepRun[] = recipe.steps.map((step, index) => ({
		label: `Step ${index + 1}: ${step.block}`,
		command: commandOf(step),
		state: index < done ? 'done' : 'notStarted',
		ms: index < done ? 2000 + index * 1000 : null
	}));
	return {
		id: '1800000000-ab12',
		recipe,
		level: recipe.level,
		branch: 'feature/avatars',
		started: 1_800_000_000,
		ended: null,
		steps,
		at: done,
		stop: null,
		outcome: null,
		restore: '1800000000-ab12',
		lines: [{ step: null, kind: 'note', text: 'restore point refs/spagitty/restore/1800000000-ab12 saved' }],
		...more
	};
}

export function viewOf(run: Run, more: Partial<RunView> = {}): RunView {
	return { repo: '/repos/thing', run, live: true, working: false, ...more };
}

/** A run stopped on the merge's conflict, as the artboard has it. */
export function conflicted(): RunView {
	const run = runOf(SYNC, 2);
	const stop: Stop = {
		step: 2,
		kind: 'conflict',
		title: 'Merge stopped on 1 conflict',
		text: 'CONFLICT (content): Merge conflict in crates/spagitty-core/src/avatars.rs',
		paths: ['crates/spagitty-core/src/avatars.rs'],
		explained: null,
		choices: ['openMerger', 'openConflicts', 'continue', 'abort', 'end'],
		operation: 'merge',
		source: 'main',
		inMerger: false
	};
	run.steps[2].state = 'waiting';
	run.steps[2].ms = 9000;
	run.stop = stop;
	run.lines.push(
		{ step: 0, kind: 'command', text: '$ git stash push -u' },
		{ step: 0, kind: 'out', text: 'Saved working directory and index state WIP on feature/avatars' },
		{ step: 2, kind: 'command', text: '$ git merge --no-ff --no-edit main' },
		{ step: 2, kind: 'error', text: 'CONFLICT (content): Merge conflict in crates/spagitty-core/src/avatars.rs' },
		{ step: 2, kind: 'note', text: 'Merge stopped on 1 conflict' }
	);
	return viewOf(run);
}

/** Settings › Agents with one command-line agent that runs and two models. */
export function agentsSnapshot(consent: string[] = []): AgentsSnapshot {
	const jobs = { review: true, merge: true, farm: true };
	return {
		codexFullAccess: false,
		omp: { model: '', profile: '' },
		local: [
			{
				id: 'claude',
				name: 'Claude Code',
				provider: 'claudeCode',
				availability: { state: 'available', path: '/usr/bin/claude', version: '2.1.0' },
				jobs,
				custom: false,
				definition: {} as never
			},
			{
				id: 'codex',
				name: 'Codex',
				provider: 'codex',
				availability: { state: 'missing' },
				jobs,
				custom: false,
				definition: {} as never
			}
		],
		remote: [
			{
				id: 'sonnet',
				name: 'Sonnet',
				provider: 'anthropic',
				base: '',
				model: 'claude-sonnet-5-5',
				jobs,
				tokensPerRun: null,
				tokensPerDay: null,
				minutes: null,
				keyEnd: 'abcd',
				spent: 0,
				spentDay: 0,
				local: false,
				providerLabel: 'Anthropic'
			},
			{
				id: 'ollama',
				name: 'Local model',
				provider: 'compatible',
				base: 'http://localhost:11434',
				model: 'qwen',
				jobs,
				tokensPerRun: null,
				tokensPerDay: null,
				minutes: null,
				keyEnd: null,
				spent: 0,
				spentDay: 0,
				local: true,
				providerLabel: 'An OpenAI-compatible endpoint'
			}
		],
		defaults: { review: null, reviewLevel: 'stepByStep', merge: null, mergeLevel: 'stepByStep' },
		notify: { waiting: true, finished: true, stopped: true },
		rules: { agents: null, consent, highest: 'signOff', neverUnattended: ['main'], verdicts: false, markComments: true },
		offer: []
	};
}

/** Let promises, and the pause after an edit, go by. */
export async function settle(ms = 160): Promise<void> {
	for (let i = 0; i < 4; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	await new Promise((done) => setTimeout(done, ms));
	await Promise.resolve();
}
