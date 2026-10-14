// SPDX-License-Identifier: GPL-3.0-or-later

/** Mirrors hooks::blocks and hooks::builder in spagitty-core, and src-tauri's Builder. */
import type { Hooks } from '../types';

export type Stage = 'pre-commit' | 'commit-msg' | 'pre-push';
export const STAGES: Stage[] = ['pre-commit', 'commit-msg', 'pre-push'];
export type Action =
	| { kind: 'tests'; paths: string[]; command: string }
	| { kind: 'format'; preset: string; paths: string[]; command: string; restage: boolean }
	| { kind: 'message'; style: string; types: string[]; scopeRequired: boolean; subjectMax: number; pattern: string; ticketPattern: string }
	| { kind: 'script'; command: string }
	| { kind: 'secrets'; patterns: string[] }
	| { kind: 'largeFiles'; bytes: number }
	| { kind: 'protectBranches'; branches: string[] };

export interface Block {
	id: string;
	label: string;
	stage: Stage;
	enabled: boolean;
	warn: boolean;
	folder: string;
	timeoutSeconds: number;
	skipMarkdown: boolean;
	skipWip: boolean;
	action: Action;
}

export interface Definition { version: number; shared: boolean; activeStages: Stage[]; blocks: Block[] }
export interface HookEvent {
	t: 'run' | 'subject' | 'start' | 'match' | 'end' | 'skip' | 'restaged' | 'done';
	id?: string;
	label?: string;
	at?: string;
	exit?: number;
	ms?: number;
	warn?: boolean;
	why?: string;
	files?: string[] | number;
	blocks?: { id: string; label: string }[];
}
export interface Run { id: string; stage: string; subject: string; at: string; exit: number | null; events: HookEvent[] }
export interface Builder { definition: Definition; info: Hooks; runs: Run[]; staged: string[]; installed: boolean }
export interface Tried { passed: boolean; output: string }

export const CATALOGUE: { kind: Action['kind']; label: string; detail: string; tone: string }[] = [
	{ kind: 'tests', label: 'Run tests when paths change', detail: 'Only when the diff touches a module', tone: 'var(--lane-5)' },
	{ kind: 'format', label: 'Format & lint', detail: 'spotless, prettier, eslint, ruff, gofmt, rustfmt', tone: 'var(--warn)' },
	{ kind: 'message', label: 'Commit message rules', detail: 'Conventional, ticket, regex', tone: 'var(--lane-4)' },
	{ kind: 'script', label: 'Run a script', detail: 'Any command or file in the repo', tone: 'var(--muted)' },
	{ kind: 'secrets', label: 'Block secrets', detail: 'Keys and tokens in the staged diff', tone: 'var(--muted)' },
	{ kind: 'largeFiles', label: 'Large files', detail: 'Stop files over a size', tone: 'var(--muted)' },
	{ kind: 'protectBranches', label: 'Protect branches', detail: 'No direct commit or push', tone: 'var(--muted)' }
];

export const PRESETS: Record<string, string> = {
	prettier: 'npx prettier --write', eslint: 'npx eslint --fix', ruff: 'ruff check --fix',
	gofmt: 'gofmt -w', rustfmt: 'rustfmt', spotless: './mvnw spotless:apply', custom: ''
};

let next = 0;
export function makeBlock(kind: Action['kind'], stage: Stage = 'pre-commit'): Block {
	const actions: Record<Action['kind'], Action> = {
		tests: { kind: 'tests', paths: ['**'], command: 'npm test' },
		format: { kind: 'format', preset: 'prettier', paths: ['**'], command: PRESETS.prettier, restage: false },
		message: { kind: 'message', style: 'conventional', types: ['feat', 'fix', 'docs', 'refactor', 'perf', 'test', 'chore'], scopeRequired: false, subjectMax: 72, pattern: '', ticketPattern: '' },
		script: { kind: 'script', command: '' }, secrets: { kind: 'secrets', patterns: [] },
		largeFiles: { kind: 'largeFiles', bytes: 10 * 1024 * 1024 }, protectBranches: { kind: 'protectBranches', branches: ['main'] }
	};
	return { id: `block-${Date.now()}-${++next}`, label: CATALOGUE.find((item) => item.kind === kind)!.label,
		stage: kind === 'message' ? 'commit-msg' : stage, enabled: true, warn: false, folder: '.', timeoutSeconds: 300,
		skipMarkdown: false, skipWip: false, action: actions[kind] };
}

export function steps(run: Run): { id: string; label: string; state: string; ms: number; startedAt: string; why: string; matched: number; restaged: string[] }[] {
	const result: ReturnType<typeof steps> = (run.events.find((event) => event.t === 'run')?.blocks ?? []).map((block) => ({ ...block, state: 'queued', ms: 0, startedAt: '', why: '', matched: 0, restaged: [] }));
	for (const event of run.events) {
		if (!event.id) continue;
		let step = result.find((step) => step.id === event.id);
		if (!step) { step = { id: event.id, label: event.label ?? event.id, state: 'queued', ms: 0, startedAt: '', why: '', matched: 0, restaged: [] }; result.push(step); }
		if (event.t === 'start') { step.state = 'running'; step.startedAt = event.at ?? run.at; step.label = event.label ?? step.label; }
		if (event.t === 'end') { step.state = event.exit === 0 ? 'passed' : event.warn ? 'warning' : 'failed'; step.ms = event.ms ?? 0; }
		if (event.t === 'skip') { step.state = 'skipped'; step.why = event.why ?? ''; }
		if (event.t === 'match' && typeof event.files === 'number') step.matched = event.files;
		if (event.t === 'restaged' && Array.isArray(event.files)) step.restaged.push(...event.files);
	}
	if (run.exit !== null) for (const step of result) {
		if (step.state === 'queued') { step.state = 'skipped'; step.why = 'an earlier block stopped the run'; }
	}
	return result;
}
