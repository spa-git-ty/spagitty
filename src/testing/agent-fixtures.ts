// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Agents and assignments for the tests (2.0). Outside `src/lib`, so it is
 * scaffolding rather than product, as the other fixtures are.
 */

import type {
	AgentsSnapshot,
	Assignment,
	LocalAgent,
	Proposal,
	RemoteAgent,
	RepoRules,
	Step
} from '$lib/agents/types';

export function aLocal(overrides: Partial<LocalAgent> = {}): LocalAgent {
	return {
		id: 'claude',
		name: 'Claude Code',
		provider: 'claudeCode',
		availability: { state: 'available', path: '/usr/bin/claude', version: '2.1.259' },
		jobs: { review: true, merge: true, farm: true },
		custom: false,
		definition: {
			id: 'claude',
			provider: 'claudeCode',
			displayName: 'Claude Code',
			executable: '/usr/bin/claude',
			capabilities: ['review'],
			inputMode: 'cliPrompt',
			role: 'architect',
			extraArgs: [],
			enabled: true,
			traits: {
				headless: true,
				streaming: true,
				resumableSessions: true,
				structuredOutput: true,
				toolUse: true
			}
		},
		...overrides
	};
}

export function aMissing(id: string, name: string): LocalAgent {
	return aLocal({ id, name, availability: { state: 'missing' } });
}

export function aRemote(overrides: Partial<RemoteAgent> = {}): RemoteAgent {
	return {
		id: 'api-1',
		name: 'Anthropic',
		provider: 'anthropic',
		base: '',
		model: 'claude-sonnet-5-5',
		jobs: { review: true, merge: true, farm: false },
		tokensPerRun: 40000,
		tokensPerDay: 400000,
		minutes: null,
		keyEnd: '7Qx2',
		spent: 0,
		spentDay: 0,
		local: false,
		providerLabel: 'Anthropic',
		...overrides
	};
}

export function someRules(overrides: Partial<RepoRules> = {}): RepoRules {
	return {
		agents: null,
		consent: [],
		highest: 'signOff',
		neverUnattended: ['main', 'master'],
		verdicts: false,
		markComments: true,
		...overrides
	};
}

export function aSnapshot(overrides: Partial<AgentsSnapshot> = {}): AgentsSnapshot {
	return {
		codexFullAccess: false,
		agyAutoApprove: false,
		omp: { model: '', profile: '' },
		local: [aLocal(), aMissing('cursor', 'Cursor')],
		remote: [aRemote()],
		defaults: { review: 'claude', reviewLevel: 'stepByStep', merge: null, mergeLevel: 'stepByStep' },
		notify: { waiting: true, finished: true, stopped: true },
		rules: someRules(),
		offer: [],
		...overrides
	};
}

/** Nothing set up: detection found nothing, no API agents. */
export function nothingSetUp(): AgentsSnapshot {
	return aSnapshot({
		local: [aMissing('claude', 'Claude Code'), aMissing('codex', 'Codex')],
		remote: [],
		defaults: { review: null, reviewLevel: 'stepByStep', merge: null, mergeLevel: 'stepByStep' }
	});
}

export function aStep(overrides: Partial<Step> = {}): Step {
	return {
		index: 0,
		kind: { kind: 'read' },
		label: 'Read the pull request · 4 files',
		state: 'done',
		startedAt: 100,
		endedAt: 112,
		gate: null,
		events: [],
		sent: [],
		refused: [],
		note: null,
		command: null,
		checks: [],
		tokens: { input: 0, output: 0 },
		...overrides
	};
}

export function aFinding(overrides: Partial<Proposal> = {}, finding: Partial<{ path: string; line: number; sure: boolean; severity: 'high' | 'medium' | 'low'; body: string }> = {}): Proposal {
	return {
		id: 'p2-0',
		step: 2,
		body: {
			kind: 'comment',
			path: 'crates/spagitty-core/src/avatars.rs',
			line: 16,
			startLine: null,
			side: 'new',
			severity: 'high',
			body: 'Two windows can write this file at once. Write to a temporary file, then rename.',
			sure: true,
			...finding
		},
		sure: finding.sure ?? true,
		state: 'proposed',
		decidedBy: null,
		why: null,
		stale: false,
		...overrides
	};
}

/** A review of #214, waiting at the first file's gate with two findings. */
export function aReview(overrides: Partial<Assignment> = {}): Assignment {
	return {
		id: 'review-1',
		repo: '/work/spagitty',
		job: 'review',
		agent: { id: 'claude', name: 'Claude Code', reach: 'local', version: '2.1.259', provider: 'claudeCode', model: null },
		level: 'stepByStep',
		levels: [],
		target: {
			kind: 'review',
			host: 'github.com',
			owner: 'spa-git-ty',
			name: 'spagitty',
			number: 214,
			title: 'Cache avatars on disk instead of in memory',
			base: 'aaa',
			head: 'bbb',
			target: 'main'
		},
		lands: false,
		state: 'waiting',
		sentence: 'Waiting for you: 2 findings on avatars.rs',
		note: '',
		steps: [
			aStep(),
			aStep({ index: 1, kind: { kind: 'plan' }, label: 'Plan · 4 files', startedAt: 112, endedAt: 143 }),
			aStep({
				index: 2,
				kind: { kind: 'file', path: 'crates/spagitty-core/src/avatars.rs' },
				label: 'avatars.rs · 2 findings',
				state: 'waiting',
				gate: 'file',
				startedAt: 143,
				endedAt: null
			})
		],
		proposals: [
			aFinding(),
			aFinding({ id: 'p2-1', sure: false }, { line: 15, severity: 'low', sure: false, body: 'Offline, a stale file now fails the avatar.' })
		],
		planned: 7,
		tokens: { input: 0, output: 0 },
		startedAt: 100,
		endedAt: null,
		reason: null,
		tookOver: null,
		lastAct: null,
		pausing: false,
		quietSince: null,
		...overrides
	};
}

/** A merge of feat/tab-drag into main, one conflict proposed. */
export function aMerge(overrides: Partial<Assignment> = {}): Assignment {
	return {
		...aReview(),
		id: 'merge-1',
		job: 'merge',
		agent: { id: 'codex', name: 'Codex', reach: 'local', version: '0.46.0', provider: 'codex', model: null },
		level: 'signOff',
		target: { kind: 'merge', a: 'main', b: 'feat/tab-drag', aTip: 'a1', bTip: 'b1', base: 'c0', strategy: 'merge', into: 'main' },
		state: 'working',
		sentence: 'Proposing a resolution · conflict 2 of 4',
		steps: [
			aStep({ label: 'Read the merge · 4 conflicts in 3 files' }),
			aStep({
				index: 1,
				kind: { kind: 'conflict', path: 'src/lib/chrome/Tabs.svelte', region: 0 },
				label: 'Tabs.svelte · Edit'
			}),
			aStep({
				index: 2,
				kind: { kind: 'conflict', path: 'src/lib/chrome/Tabs.svelte', region: 1 },
				label: 'Tabs.svelte',
				state: 'running',
				endedAt: null
			})
		],
		proposals: [
			{
				id: 'p1-0',
				step: 1,
				body: {
					kind: 'resolution',
					path: 'src/lib/chrome/Tabs.svelte',
					region: 0,
					choice: { mode: 'edit', text: '<Tab {tab} pinned={tab.pinned} draggable="true" />' },
					why: 'Keeps pinned from main and dragging from feat/tab-drag.'
				},
				sure: true,
				state: 'applied',
				decidedBy: 'agent',
				why: null,
				stale: false
			}
		],
		planned: 7,
		...overrides
	};
}
