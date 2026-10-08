// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Agents in Review and Merger (2.0), as the backend sends them.
 *
 * Mirrors of `src-tauri/src/agents.rs` and `spagitty_farm::assign`. Every
 * field is camelCase on the wire.
 */

import type { AgentAvailability, AgentDefinition, AgentProvider } from '$lib/farm/types';
import type { Choice } from '$lib/resolver/model';

export type Job = 'review' | 'merge';

/** How far the agent goes alone. */
export type Level = 'suggest' | 'stepByStep' | 'signOff' | 'unattended';

export type Gate = 'plan' | 'file' | 'verdict' | 'send' | 'conflict' | 'checks' | 'land';

export type ModelProvider = 'anthropic' | 'openAi' | 'google' | 'compatible';

export interface Jobs {
	review: boolean;
	merge: boolean;
	farm: boolean;
}

export interface LocalAgent {
	id: string;
	name: string;
	provider: AgentProvider;
	availability: AgentAvailability;
	jobs: Jobs;
	custom: boolean;
	definition: AgentDefinition;
}

export interface RemoteAgent {
	id: string;
	name: string;
	provider: ModelProvider;
	base: string;
	model: string;
	jobs: Jobs;
	tokensPerRun: number | null;
	tokensPerDay: number | null;
	minutes: number | null;
	/** The stored key's last four characters. Never the key. */
	keyEnd: string | null;
	spent: number;
	spentDay: number;
	/** The endpoint is on this machine: nothing leaves it. */
	local: boolean;
	providerLabel: string;
}

export interface Defaults {
	review: string | null;
	reviewLevel: Level;
	merge: string | null;
	mergeLevel: Level;
}

export interface Notify {
	waiting: boolean;
	finished: boolean;
	stopped: boolean;
}

export interface RepoRules {
	/** Agents that may be used here; null is all of them. */
	agents: string[] | null;
	/** Providers this repository's code may go to. */
	consent: string[];
	highest: Level;
	neverUnattended: string[];
	verdicts: boolean;
	markComments: boolean;
}

export interface AgentsSnapshot {
	codexFullAccess: boolean;
	local: LocalAgent[];
	remote: RemoteAgent[];
	defaults: Defaults;
	notify: Notify;
	rules: RepoRules | null;
	offer: AgentDefinition[];
}

export interface Tested {
	ok: boolean;
	said: string;
	ms: number;
}

export interface RemoteInput {
	id?: string | null;
	name: string;
	provider: ModelProvider;
	base: string;
	model: string;
	jobs?: Jobs;
	tokensPerRun: number | null;
	tokensPerDay: number | null;
	minutes: number | null;
	/** A new key, only ever on the way in. */
	key?: string | null;
}

export interface Probe {
	id?: string | null;
	provider: ModelProvider;
	base: string;
	model: string;
	key?: string | null;
}

// ── Assignments ──────────────────────────────────────────────────────────

export type Reach = 'local' | 'remote';

export interface AgentRef {
	id: string;
	name: string;
	reach: Reach;
	version: string | null;
	provider: string;
	model: string | null;
}

export type Who = 'person' | 'agent';

export type Target =
	| {
			kind: 'review';
			host: string;
			owner: string;
			name: string;
			number: number;
			title: string;
			base: string;
			head: string;
			target: string;
	  }
	| {
			kind: 'merge';
			a: string;
			b: string;
			aTip: string;
			bTip: string;
			base: string;
			strategy: string;
			/** The branch that receives the merge, when it is not `a`. */
			into: string;
	  };

export type Life = 'starting' | 'working' | 'waiting' | 'paused' | 'stopped' | 'failed' | 'done';

export type StepKind =
	| { kind: 'read' }
	| { kind: 'plan' }
	| { kind: 'file'; path: string }
	| { kind: 'verdict' }
	| { kind: 'conflict'; path: string; region: number }
	| { kind: 'checks' }
	| { kind: 'why'; proposal: string }
	| { kind: 'last' };

export type StepState = 'running' | 'done' | 'waiting' | 'failed' | 'superseded';

export interface CheckRun {
	command: string;
	passed: boolean;
	output: string;
	durationMs: number;
}

export interface Tokens {
	input: number;
	output: number;
}

export interface Step {
	index: number;
	kind: StepKind;
	label: string;
	state: StepState;
	startedAt: number;
	endedAt: number | null;
	gate: Gate | null;
	events: string[];
	sent: string[];
	refused: string[];
	note: string | null;
	command: string | null;
	checks: CheckRun[];
	tokens: Tokens;
}

export type Severity = 'high' | 'medium' | 'low';
export type Verdict = 'comment' | 'approve' | 'requestChanges';

export interface Finding {
	path: string;
	line: number;
	startLine: number | null;
	side: 'old' | 'new';
	severity: Severity;
	body: string;
	sure: boolean;
}

export interface PlanItem {
	path: string;
	why: string;
}

export type ProposalBody =
	| { kind: 'plan'; files: PlanItem[]; lookFor: string }
	| ({ kind: 'comment' } & Finding)
	| { kind: 'verdict'; summary: string; verdict: Verdict; sure: boolean }
	| { kind: 'resolution'; path: string; region: number; choice: Choice; why: string };

export type ProposalState = 'proposed' | 'accepted' | 'edited' | 'dismissed' | 'superseded' | 'applied';

export interface Proposal {
	id: string;
	step: number;
	body: ProposalBody;
	sure: boolean;
	state: ProposalState;
	decidedBy: Who | null;
	why: string | null;
	stale: boolean;
}

export interface LevelChange {
	from: Level;
	to: Level;
	by: Who;
	at: number;
}

export type LastAct = { kind: 'send'; verdict: Verdict; body: string } | { kind: 'land' };

export interface Assignment {
	id: string;
	repo: string;
	job: Job;
	agent: AgentRef;
	level: Level;
	levels: LevelChange[];
	target: Target;
	lands: boolean;
	state: Life;
	sentence: string;
	note: string;
	steps: Step[];
	proposals: Proposal[];
	planned: number;
	tokens: Tokens;
	startedAt: number;
	endedAt: number | null;
	reason: string | null;
	tookOver: string | null;
	lastAct: LastAct | null;
	pausing: boolean;
	quietSince: number | null;
}

/** What the engine is handed with a review. */
export interface ThreadBrief {
	path: string | null;
	line: number | null;
	author: string;
	body: string;
	resolved: boolean;
}

export interface MergeRegion {
	index: number;
	start: number;
	end: number;
	a: string[];
	b: string[];
	base: string[] | null;
	aFrom: string | null;
	bFrom: string | null;
}

export interface MergeFile {
	path: string;
	merged: string[];
	eol: boolean;
	whole: boolean;
	regions: MergeRegion[];
}

export type Work =
	| {
			job: 'review';
			description: string;
			threads: ThreadBrief[];
			checks: string;
			conflictFixes: string[];
	  }
	| { job: 'merge'; files: MergeFile[] };

export interface StartRequest {
	repo: string;
	agent: string;
	level: Level;
	note: string;
	target: Target;
	work: Work;
	lands: boolean;
	/** Carry on this assignment from its last finished step. */
	resume?: string | null;
}

export type Control =
	| { kind: 'pause' }
	| { kind: 'resume' }
	| { kind: 'stop' }
	| { kind: 'takeOver' }
	| { kind: 'level'; level: Level }
	| { kind: 'tell'; text: string }
	| { kind: 'continue' }
	| { kind: 'redo'; note: string }
	| { kind: 'askWhy'; proposal: string }
	| { kind: 'decide'; proposal: string; state: ProposalState; choice?: Choice | null }
	| { kind: 'plan'; files: string[] }
	| { kind: 'moved' }
	| { kind: 'carryOn' }
	| { kind: 'acted'; ok: boolean; message?: string };

/** What the Assign popover hands back. */
export interface Assigned {
	agent: string;
	level: Level;
	note: string;
	/** For a merge: resolve, check and land, rather than resolve only. */
	lands: boolean;
}

export interface Failure {
	kind: 'refused' | 'consent' | 'notFound' | 'keychain' | 'model' | 'git' | 'io';
	message: string;
}
