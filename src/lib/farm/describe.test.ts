// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The words the Farm screen uses (FEAT-073).
 *
 * Copy is asserted here rather than in the component tests, which is what keeps
 * those about behaviour. Two of these are load-bearing rather than cosmetic and
 * are called out where they appear:
 *
 * * an unverified task must never read as a pass;
 * * a task with no declared paths must say that it runs alone, because that is
 *   why a farm with three ready tasks starts one.
 */

import { describe, expect, it } from 'vitest';
import {
	AUTONOMY_LEVELS,
	availabilityLabel,
	duration,
	eventLine,
	FARM_STATUS_LABELS,
	isMoving,
	originLine,
	originMark,
	PROVIDER_LABELS,
	TASK_KIND_LABELS,
	TASK_STATUS_LABELS,
	taskStatusLabel,
	tone,
	verificationLine,
	waitingOn
} from './describe';
import type { AgentProvider, FarmEvent, FarmStatus, TaskKind, TaskStatus, Verification } from './types';

/** Every member of each closed set, so a new one cannot arrive unlabelled. */
const ALL_TASK_STATUSES: TaskStatus[] = [
	'draft',
	'ready',
	'assigned',
	'running',
	'waiting',
	'blocked',
	'review',
	'verification',
	'done',
	'failed',
	'cancelled'
];

const ALL_FARM_STATUSES: FarmStatus[] = [
	'idle',
	'planning',
	'running',
	'paused',
	'blocked',
	'reviewing',
	'completed',
	'failed',
	'cancelled'
];

const ALL_KINDS: TaskKind[] = [
	'architecture',
	'backend',
	'frontend',
	'testing',
	'documentation',
	'research',
	'review',
	'integration',
	'general'
];

const ALL_PROVIDERS: AgentProvider[] = ['claudeCode', 'codex', 'cursor', 'ohMyPi', 'custom'];

describe('every closed set is labelled', () => {
	it('labels every task status', () => {
		for (const status of ALL_TASK_STATUSES) {
			expect(TASK_STATUS_LABELS[status], status).toBeTruthy();
			expect(taskStatusLabel(status)).toBe(TASK_STATUS_LABELS[status]);
		}
	});

	it('labels every farm status', () => {
		for (const status of ALL_FARM_STATUSES) {
			expect(FARM_STATUS_LABELS[status], status).toBeTruthy();
		}
	});

	it('labels every task kind', () => {
		for (const kind of ALL_KINDS) {
			expect(TASK_KIND_LABELS[kind], kind).toBeTruthy();
		}
	});

	it('labels every provider with the name people use', () => {
		for (const provider of ALL_PROVIDERS) {
			expect(PROVIDER_LABELS[provider], provider).toBeTruthy();
		}
		expect(PROVIDER_LABELS.claudeCode).toBe('Claude Code');
		expect(PROVIDER_LABELS.ohMyPi).toBe('Oh My Pi');
	});

	it('describes every autonomy level in terms of where the human is', () => {
		expect(AUTONOMY_LEVELS.map((level) => level.id)).toEqual([
			'manual',
			'assisted',
			'semiAuto',
			'auto',
			'yolo'
		]);
		for (const level of AUTONOMY_LEVELS) {
			expect(level.detail.length, level.id).toBeGreaterThan(10);
		}
	});
});

describe('tone', () => {
	it('gives every status one of the four tones', () => {
		for (const status of ALL_TASK_STATUSES) {
			expect(['done', 'active', 'stuck', 'idle'], status).toContain(tone(status));
		}
	});

	it('reads a finished task as done and a broken one as stuck', () => {
		expect(tone('done')).toBe('done');
		expect(tone('failed')).toBe('stuck');
		expect(tone('blocked')).toBe('stuck');
	});

	it('reads everything an agent is doing as active', () => {
		for (const status of ['running', 'verification', 'review', 'assigned'] as TaskStatus[]) {
			expect(tone(status), status).toBe('active');
		}
	});

	it('reads a cancelled task as neither done nor stuck', () => {
		// Cancelling is not a failure — the farm did what it was told.
		expect(tone('cancelled')).toBe('idle');
	});

	it('marks only the statuses that move without anyone pressing anything', () => {
		expect(isMoving('running')).toBe(true);
		expect(isMoving('verification')).toBe(true);
		expect(isMoving('assigned')).toBe(true);
		// Review waits for a reviewer that may be a person.
		expect(isMoving('review')).toBe(false);
		expect(isMoving('ready')).toBe(false);
	});
});

describe('availability', () => {
	it('shows the version when an agent answered', () => {
		expect(
			availabilityLabel({ state: 'available', path: '/usr/bin/claude', version: '1.2.3' })
		).toBe('1.2.3');
	});

	it('falls back to a word when the version was empty', () => {
		expect(availabilityLabel({ state: 'available', path: '/x', version: '' })).toBe('Detected');
	});

	it('says installed-and-broken differently from not installed', () => {
		const broken = availabilityLabel({ state: 'broken', path: '/x', reason: 'exited with 127' });
		expect(broken).toContain('exited with 127');
		expect(broken).not.toBe(availabilityLabel({ state: 'missing' }));
		expect(availabilityLabel({ state: 'missing' })).toBe('Not installed');
	});
});

describe('verificationLine', () => {
	function verification(over: Partial<Verification>): Verification {
		return { results: [], passed: false, unverified: false, ...over };
	}

	it('says nothing has run when nothing has', () => {
		expect(verificationLine(null)).toBe('Not run');
	});

	/** The one sentence the whole feature rests on not getting wrong. */
	it('never reads an unverified task as a pass', () => {
		const line = verificationLine(verification({ unverified: true }));
		expect(line).toContain('Nothing was checked');
		expect(line.toLowerCase()).not.toContain('passed');
	});

	it('counts the checks that passed', () => {
		expect(
			verificationLine(
				verification({
					passed: true,
					results: [
						{ command: 'cargo test', passed: true, output: '', durationMs: 1 },
						{ command: 'npm test', passed: true, output: '', durationMs: 1 }
					]
				})
			)
		).toBe('2 checks passed');
	});

	it('uses the singular for one check', () => {
		expect(
			verificationLine(
				verification({
					passed: true,
					results: [{ command: 'cargo test', passed: true, output: '', durationMs: 1 }]
				})
			)
		).toBe('1 check passed');
	});

	it('names the commands that failed', () => {
		expect(
			verificationLine(
				verification({
					results: [
						{ command: 'cargo test', passed: true, output: '', durationMs: 1 },
						{ command: 'npm run check', passed: false, output: '', durationMs: 1 }
					]
				})
			)
		).toBe('Failed: npm run check');
	});
});

describe('waitingOn', () => {
	const byId = new Map([
		['TASK-0001', { status: 'done' as TaskStatus }],
		['TASK-0002', { status: 'running' as TaskStatus }]
	]);

	it('names the dependency that has not finished', () => {
		expect(
			waitingOn({ dependsOn: ['TASK-0002'], allowedPaths: ['src/**'] }, byId)
		).toBe('Waiting for TASK-0002');
	});

	it('says nothing when every dependency is done and paths are declared', () => {
		expect(waitingOn({ dependsOn: ['TASK-0001'], allowedPaths: ['src/**'] }, byId)).toBeNull();
	});

	/** Why a farm with three ready tasks starts one. */
	it('explains that an undeclared task runs alone', () => {
		expect(waitingOn({ dependsOn: [], allowedPaths: [] }, byId)).toContain('runs on its own');
	});

	it('reports the dependency before the path warning', () => {
		// Both are true; the one that will resolve on its own comes first.
		expect(waitingOn({ dependsOn: ['TASK-0002'], allowedPaths: [] }, byId)).toContain(
			'Waiting for'
		);
	});

	it('treats a dependency that is not in the map as unfinished', () => {
		expect(waitingOn({ dependsOn: ['TASK-9999'], allowedPaths: ['a'] }, byId)).toContain(
			'TASK-9999'
		);
	});
});

describe('eventLine', () => {
	/** One line per event kind, so a new kind cannot render as blank. */
	const events: FarmEvent[] = [
		{ kind: 'farmStatusChanged', status: 'running' },
		{ kind: 'taskCreated', task: 'TASK-0001', title: 'Investigate' },
		{ kind: 'taskStatusChanged', task: 'TASK-0001', status: 'done', note: null },
		{ kind: 'taskAssigned', task: 'TASK-0001', agent: 'codex' },
		{ kind: 'agentStarted', run: 'r', task: 'TASK-0001', agent: 'codex', command: 'codex exec' },
		{ kind: 'agentOutput', run: 'r', task: 'TASK-0001', line: 'reading files' },
		{ kind: 'agentStopped', run: 'r', task: 'TASK-0001', ok: true, reason: null },
		{ kind: 'verificationStarted', task: 'TASK-0001', command: 'cargo test' },
		{ kind: 'verificationFinished', task: 'TASK-0001', command: 'cargo test', passed: true, output: '' },
		{ kind: 'reviewRequested', task: 'TASK-0001', reviewer: 'claude' },
		{ kind: 'reviewCompleted', task: 'TASK-0001', reviewer: 'claude', approved: true, summary: '' },
		{ kind: 'mergeRequested', task: 'TASK-0001', branch: 'spagitty-farm/TASK-0001/codex' },
		{ kind: 'mergeCompleted', task: 'TASK-0001', branch: 'b', ok: true, error: null },
		{ kind: 'workspaceChanged', task: 'TASK-0001', path: '/x', created: true },
		{ kind: 'taskProposed', from: 'TASK-0001', title: 'Add the store' },
		{ kind: 'failed', message: 'something went wrong' }
	];

	it('renders every kind as a non-empty line', () => {
		for (const event of events) {
			expect(eventLine(event), event.kind).toBeTruthy();
		}
	});

	it('names the task and the agent where it has them', () => {
		expect(eventLine(events[4])).toBe('codex started on TASK-0001');
		expect(eventLine(events[1])).toBe('TASK-0001 created — Investigate');
	});

	it('includes the reason a status changed when there is one', () => {
		expect(
			eventLine({
				kind: 'taskStatusChanged',
				task: 'TASK-0001',
				status: 'blocked',
				note: 'Verification failed'
			})
		).toContain('Verification failed');
	});

	it('says whether a check passed or failed', () => {
		expect(eventLine(events[8])).toContain('passed');
		expect(
			eventLine({
				kind: 'verificationFinished',
				task: 'TASK-0001',
				command: 'cargo test',
				passed: false,
				output: ''
			})
		).toContain('failed');
	});

	it('distinguishes an approval from a change request', () => {
		expect(eventLine(events[10])).toContain('approved');
		expect(
			eventLine({
				kind: 'reviewCompleted',
				task: 'TASK-0001',
				reviewer: 'claude',
				approved: false,
				summary: ''
			})
		).toContain('asked for changes');
	});

	it('carries an agent failure reason', () => {
		expect(
			eventLine({
				kind: 'agentStopped',
				run: 'r',
				task: 'TASK-0001',
				ok: false,
				reason: 'exited with 4'
			})
		).toContain('exited with 4');
	});

	it('says when a merge did not apply', () => {
		expect(
			eventLine({
				kind: 'mergeCompleted',
				task: 'TASK-0001',
				branch: 'b',
				ok: false,
				error: 'conflict'
			})
		).toContain('conflict');
	});

	it('reports a created worktree differently from a removed one', () => {
		expect(eventLine(events[13])).toContain('created');
		expect(
			eventLine({ kind: 'workspaceChanged', task: 'TASK-0001', path: '/x', created: false })
		).toContain('removed');
	});

	it('passes an agent transcript line through unchanged', () => {
		expect(eventLine(events[5])).toBe('reading files');
	});
});

describe('duration', () => {
	it('says nothing for a run that has not finished', () => {
		expect(duration(null)).toBe('');
		expect(duration(undefined)).toBe('');
	});

	it('uses milliseconds below a second', () => {
		expect(duration(430)).toBe('430ms');
	});

	it('uses seconds below a minute', () => {
		expect(duration(4_400)).toBe('4s');
	});

	it('uses minutes and seconds above one', () => {
		expect(duration(125_000)).toBe('2m 5s');
	});

	it('handles zero rather than rendering nothing', () => {
		expect(duration(0)).toBe('0ms');
	});
});

describe('who asked for a task (FEAT-078)', () => {
	it('says it plainly, one line each', () => {
		expect(originLine({ kind: 'person' })).toBe('You added this.');
		expect(originLine({ kind: 'planned', agent: 'claude' })).toBe(
			'claude cut this out of the goal.'
		);
		expect(originLine({ kind: 'subtask', agent: 'codex', parent: 'TASK-0002' })).toBe(
			'codex cut this out of TASK-0002.'
		);
		expect(originLine({ kind: 'proposed', agent: 'claude', from: 'TASK-0003' })).toContain(
			'Nobody asked for it'
		);
	});

	it('still names the task a proposal came from when the agent is unknown', () => {
		// A farm reopened after the agent was removed still knows where the
		// proposal came from, which is the half that matters.
		const line = originLine({ kind: 'proposed', agent: null, from: 'TASK-0003' });
		expect(line).toContain('TASK-0003');
		expect(line).not.toContain('null');
	});

	it('marks only what an agent asked for', () => {
		expect(originMark({ kind: 'person' })).toBeNull();
		for (const origin of [
			{ kind: 'planned', agent: 'a' },
			{ kind: 'subtask', agent: 'a', parent: 'T-1' },
			{ kind: 'proposed', agent: 'a', from: 'T-1' }
		] as const) {
			expect(originMark(origin)).not.toBeNull();
		}
	});
});
