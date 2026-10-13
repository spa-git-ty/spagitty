// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Recipes (FEAT-133), as the backend sends them.
 *
 * Mirrors of `spagitty_core::recipes` and `src-tauri/src/recipes.rs`. Every
 * field is camelCase on the wire.
 */

/** How far a run goes without the person. The stored values of 2.0's levels. */
export type Level = 'suggest' | 'stepByStep' | 'signOff' | 'unattended';

export type PullMode = 'fastForwardOnly' | 'merge' | 'rebase';
export type MergeHow = 'commit' | 'fastForward' | 'squash';

/** A commit's message: typed, or left to the agent. */
export type Message = { kind: 'text'; text: string } | { kind: 'agent' };

/**
 * One block of a recipe, with its parameters. Mirrors `recipes::Step`: the
 * catalogue is closed, and this union is all of it.
 */
export type Step =
	| { block: 'stash'; params: { untracked: boolean } }
	| { block: 'pop'; params: Record<string, never> }
	| { block: 'commit'; params: { all: boolean; message: Message } }
	| { block: 'amend'; params: { message: Message | null } }
	| { block: 'fetch'; params: { remote: string } }
	| { block: 'pull'; params: { remote: string; mode: PullMode } }
	| { block: 'push'; params: { remote: string } }
	| { block: 'merge'; params: { branch: string; how: MergeHow } }
	| { block: 'rebase'; params: { onto: string } }
	| { block: 'cherryPick'; params: { commits: string[] } }
	| { block: 'switch'; params: { branch: string } }
	| { block: 'newBranch'; params: { name: string; from: string } }
	| { block: 'deleteBranch'; params: { branch: string } }
	| { block: 'squash'; params: { count: number; message: Message } }
	| { block: 'reword'; params: { message: Message } }
	| { block: 'check'; params: { command: string } }
	| { block: 'ask'; params: Record<string, never> }
	| { block: 'clean'; params: Record<string, never> };

export type BlockId = Step['block'];

/** A recipe, in the form it is saved, exported and imported in. */
export interface Recipe {
	v: number;
	name: string;
	/** The branch it runs on. Empty is whichever is checked out. */
	branch: string;
	steps: Step[];
	level: Level;
	/** The agent's help. Off, no agent is called at any point. */
	agentHelp: boolean;
	agent: string | null;
	note: string;
}

/** What is true of the repository. Mirrors `validate::World`. */
export interface World {
	branches: string[];
	remotes: string[];
	current: string | null;
	dirty: boolean;
	operation: string | null;
	upstream: boolean;
	unpushed: number;
	rewritesPushed: string[];
	protected: string[];
	checks: string[];
	agents: string[];
	highest: Level;
}

export interface Finding {
	/** The step it is about, counted from nothing. Null is the recipe. */
	step: number | null;
	severity: 'error' | 'warning';
	code: string;
	text: string;
}

/** Everything the builder shows about a stack. Mirrors `recipes::Check`. */
export interface Check {
	findings: Finding[];
	errors: number;
	labels: string[];
	commands: string[];
	/** What the agent is given. Null with its help off. */
	brief: string | null;
	script: string;
	level: Level;
	world: World;
}

export interface Listed {
	recipes: Recipe[];
	guideVersion: number;
}

export interface Proposed {
	steps: Step[];
	said: string;
}

// ── A run ────────────────────────────────────────────────────────────────

export type LineKind = 'command' | 'out' | 'error' | 'note' | 'agent';

export interface Line {
	step: number | null;
	kind: LineKind;
	text: string;
}

export type StepState = 'notStarted' | 'running' | 'done' | 'waiting' | 'skipped';

export interface StepRun {
	label: string;
	command: string;
	state: StepState;
	ms: number | null;
}

export type StopKind =
	| 'approve'
	| 'ask'
	| 'conflict'
	| 'failed'
	| 'checkFailed'
	| 'pushRejected'
	| 'notClean'
	| 'changed'
	| 'noMessage';

export type Choice = 'continue' | 'retry' | 'skip' | 'openMerger' | 'openConflicts' | 'abort' | 'end';

export interface Stop {
	step: number;
	kind: StopKind;
	title: string;
	text: string;
	paths: string[];
	explained: string | null;
	choices: Choice[];
	operation: string | null;
	source: string | null;
	inMerger: boolean;
}

export type Outcome = 'done' | 'ended' | 'undone';

export interface Run {
	id: string;
	recipe: Recipe;
	level: Level;
	branch: string;
	started: number;
	ended: number | null;
	steps: StepRun[];
	at: number;
	stop: Stop | null;
	outcome: Outcome | null;
	restore: string;
	lines: Line[];
}

/** A run as the screen is sent it. Mirrors `recipes::RunView`. */
export interface RunView {
	repo: string;
	run: Run;
	/** It can be answered and stopped. False for a record read back. */
	live: boolean;
	working: boolean;
}

export interface RestorePoint {
	id: string;
	created: number;
	recipe: string;
	branch: string | null;
	head: string;
	tips: { name: string; id: string }[];
	makes: string[];
	stashes: { id: string; message: string }[];
}
