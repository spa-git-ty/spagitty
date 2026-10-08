// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Unit tests for the Farm store (FEAT-073).
 */

import { agents as machineAgents } from '$lib/agents/store.svelte';
import { aLocal, aSnapshot } from '../../testing/agent-fixtures';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Farm, FarmSnapshot, Task } from './types';

let eventHandler: ((event: { payload: unknown }) => void) | null = null;
const unlistenMock = vi.fn();

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn((_name: string, handler: (event: { payload: unknown }) => void) => {
		eventHandler = handler;
		return Promise.resolve(unlistenMock);
	})
}));

vi.mock('./api', () => ({
	open: vi.fn(),
	exists: vi.fn(),
	taskDetail: vi.fn(() => Promise.resolve(null)),
	snapshot: vi.fn(),
	stale: vi.fn(() => Promise.resolve([])),
	failure: vi.fn((err: unknown) => ({
		kind: 'testError',
		message: typeof err === 'string' ? err : (err as Error)?.message ?? 'failed'
	}))
}));

import * as api from './api';
import { farmStore } from './store.svelte';

const apiOpen = vi.mocked(api.open);
const apiSnapshot = vi.mocked(api.snapshot);

function sampleTask(id: string, overrides: Partial<Task> = {}): Task {
	return {
		id,
		title: `Task ${id}`,
		description: 'A sample task',
		status: 'ready',
		kind: 'general',
		parent: null,
		origin: { kind: 'person' },
		priority: 'normal',
		dependsOn: [],
		allowedPaths: [],
		acceptanceCriteria: [],
		verification: [],
		verificationOverrides: false,
		assignedAgent: null,
		implementedBy: null,
		branch: null,
		worktree: null,
		attempts: 0,
		createdMs: 1000,
		updatedMs: 1000,
		note: null,
		...overrides
	};
}

function sampleFarm(tasks: Task[] = []): Farm {
	return {
		id: 'farm-1',
		repository: '/path/to/repo',
		status: 'running',
		autonomy: 'semiAuto',
		permissions: {
			writeFiles: true,
			runCommands: true,
			network: false,
			commit: true,
			push: false,
			merge: false,
			deleteBranch: false
		},
		goal: {
			id: 'goal-1',
			title: 'A big goal',
			description: 'Doing important things',
			constraints: [],
			createdMs: 1000
		},
		agents: [],
		tasks,
		verification: ['cargo test'],
		maxParallel: 2,
		maxAttempts: 3,
		createdMs: 1000,
		updatedMs: 1000
	};
}

function sampleSnapshot(tasks: Task[] = []): FarmSnapshot {
	return {
		farm: sampleFarm(tasks),
		agents: [
			{
				definition: {
					id: 'claude-1',
					provider: 'claudeCode',
					displayName: 'Claude Code',
					executable: '/usr/bin/claude',
					role: 'architect',
					capabilities: ['planning', 'coding'],
					inputMode: 'cliPrompt',
					traits: {
						resumableSessions: true,
						streaming: true,
						structuredOutput: true,
						toolUse: true,
						headless: true
					},
					enabled: true,
					extraArgs: []
				},
				availability: { state: 'available', path: '/usr/bin/claude', version: '1.0.0' }
			},
			{
				definition: {
					id: 'codex-1',
					provider: 'codex',
					displayName: 'Codex',
					executable: '/usr/bin/codex',
					role: 'backend',
					capabilities: ['coding'],
					inputMode: 'cliPrompt',
					traits: {
						resumableSessions: false,
						streaming: true,
						structuredOutput: true,
						toolUse: true,
						headless: true
					},
					enabled: false,
					extraArgs: []
				},
				availability: { state: 'available', path: '/usr/bin/codex', version: '2.0.0' }
			},
			{
				definition: {
					id: 'cursor-1',
					provider: 'cursor',
					displayName: 'Cursor',
					executable: '/usr/bin/cursor',
					role: 'frontend',
					capabilities: ['coding'],
					inputMode: 'cliPrompt',
					traits: {
						resumableSessions: false,
						streaming: true,
						structuredOutput: false,
						toolUse: true,
						headless: false
					},
					enabled: true,
					extraArgs: []
				},
				availability: { state: 'missing' }
			}
		],
		undetected: ['ohMyPi'],
		runs: [],
		policy: {
			sources: [{ path: 'AGENTS.md', authoritative: true, bytes: 42 }],
			text: '# Rules'
		},
		scoreboard: [
			{
				agent: 'claude-1',
				completed: 3,
				failed: 0,
				changesRequested: 1,
				successRate: 0.75,
				averageMs: 5000
			}
		],
		events: [],
		waiting: {}
	};
}

beforeEach(() => {
	vi.clearAllMocks();
	eventHandler = null;
	farmStore.reset();
});

describe('farmStore initial & reset state', () => {
	it('starts completely empty', () => {
		expect(farmStore.farm).toBeNull();
		expect(farmStore.tasks).toEqual([]);
		expect(farmStore.agents).toEqual([]);
		expect(farmStore.usable).toEqual([]);
		expect(farmStore.loaded).toBe(false);
		expect(farmStore.loading).toBe(false);
		expect(farmStore.error).toBeNull();
		expect(farmStore.progress).toEqual({ done: 0, total: 0 });
		expect(farmStore.needsYou).toEqual([]);
	});
});

describe('farmStore.open & getters', () => {
	it('loads snapshot and sets loaded state', async () => {
		const t1 = sampleTask('TASK-001', { status: 'done' });
		const t2 = sampleTask('TASK-002', { status: 'running' });
		const t3 = sampleTask('TASK-003', { status: 'review' });
		const snapshot = sampleSnapshot([t1, t2, t3]);

		apiOpen.mockResolvedValueOnce(snapshot);

		await farmStore.open('/my/repo');

		expect(farmStore.loaded).toBe(true);
		expect(farmStore.loading).toBe(false);
		expect(farmStore.farm?.id).toBe('farm-1');
		expect(farmStore.tasks.length).toBe(3);
		expect(farmStore.progress).toEqual({ done: 1, total: 3 });
		expect(farmStore.needsYou.map((t) => t.id)).toEqual(['TASK-003']);
		expect(farmStore.inStatus('running').map((t) => t.id)).toEqual(['TASK-002']);
		expect(farmStore.byId.get('TASK-001')?.title).toBe('Task TASK-001');

		// Usable agents filter by enabled && available
		expect(farmStore.usable.length).toBe(1);
		expect(farmStore.usable[0].definition.id).toBe('claude-1');
		// ...and by the machine's word: switched off for the farm in
		// Settings › Agents, it is not offered (2.0).
		machineAgents.reset(aSnapshot({ local: [aLocal({ id: 'claude-1', jobs: { review: true, merge: true, farm: false } })] }));
		expect(farmStore.usable).toEqual([]);
		machineAgents.reset(null);

		expect(farmStore.undetected).toEqual(['ohMyPi']);
		expect(farmStore.policy.sources.map((s) => s.path)).toEqual(['AGENTS.md']);
		expect(farmStore.scoreboard.length).toBe(1);
	});

	it('records error on failed open', async () => {
		apiOpen.mockRejectedValueOnce(new Error('Permission denied'));

		await farmStore.open('/invalid/path');

		expect(farmStore.loaded).toBe(false);
		expect(farmStore.error).toBe('Permission denied');
	});
});

describe('farmStore events & absorb', () => {
	it('absorbs agentOutput events into transcripts per task', () => {
		farmStore.absorb({
			atMs: 1_700_000_000_000,
			kind: 'agentOutput',
			task: 'TASK-001',
			run: 'run-1',
			line: 'Compiling project...'
		});
		farmStore.absorb({
			atMs: 1_700_000_000_000,
			kind: 'agentOutput',
			task: 'TASK-001',
			run: 'run-1',
			line: 'Done.'
		});
		farmStore.absorb({
			atMs: 1_700_000_000_000,
			kind: 'agentOutput',
			task: 'TASK-002',
			run: 'run-2',
			line: 'Warning: unused'
		});

		expect(farmStore.transcript('TASK-001')).toEqual(['Compiling project...', 'Done.']);
		expect(farmStore.transcript('TASK-002')).toEqual(['Warning: unused']);
		expect(farmStore.transcript('TASK-003')).toEqual([]);
	});

	it('absorbs farmStatusChanged and taskStatusChanged into local farm', async () => {
		const t1 = sampleTask('TASK-001', { status: 'ready' });
		apiOpen.mockResolvedValueOnce(sampleSnapshot([t1]));
		await farmStore.open('/repo');

		farmStore.absorb({
			atMs: 1_700_000_000_000,
			kind: 'farmStatusChanged',
			status: 'paused'
		});
		expect(farmStore.farm?.status).toBe('paused');

		farmStore.absorb({
			atMs: 1_700_000_000_000,
			kind: 'taskStatusChanged',
			task: 'TASK-001',
			status: 'running',
			note: 'Agent starting'
		});
		expect(farmStore.tasks[0].status).toBe('running');
		expect(farmStore.tasks[0].note).toBe('Agent starting');
	});
});

describe('farmStore refresh & stop', () => {
	it('refresh updates snapshot without clearing current data on failure', async () => {
		const t1 = sampleTask('TASK-001', { status: 'ready' });
		apiOpen.mockResolvedValueOnce(sampleSnapshot([t1]));
		await farmStore.open('/repo');

		// Successful refresh
		const t1Updated = sampleTask('TASK-001', { status: 'done' });
		apiSnapshot.mockResolvedValueOnce(sampleSnapshot([t1Updated]));
		await farmStore.refresh();
		expect(farmStore.tasks[0].status).toBe('done');

		// Failed refresh records error but keeps data
		apiSnapshot.mockRejectedValueOnce(new Error('Network disconnected'));
		await farmStore.refresh();
		expect(farmStore.error).toBe('Network disconnected');
		expect(farmStore.tasks.length).toBe(1);
	});

	it('does not ask the backend anything when a transcript line arrives', async () => {
		// TASK-030. A run produces thousands of these, none of which change
		// anything a snapshot reports, and each one used to restart the
		// debounce — so the refresh that did matter never ran until the agent
		// stopped talking.
		// A live subscription, not `absorb` by hand: what is being tested is
		// what the listener decides to do, and `listen` is a no-op if the store
		// is already subscribed from an earlier test.
		await farmStore.stop();
		apiOpen.mockResolvedValueOnce(sampleSnapshot([]));
		await farmStore.open('/repo');
		expect(eventHandler, 'no subscription; the rest would assert nothing').not.toBeNull();

		vi.useFakeTimers();
		try {
			apiSnapshot.mockClear();

			eventHandler?.({
				payload: { kind: 'agentOutput', run: 'r1', task: 'TASK-001', line: 'working' }
			});
			await vi.advanceTimersByTimeAsync(1000);
			expect(apiSnapshot).not.toHaveBeenCalled();
			// It is still applied locally: the pane fills as the agent talks.
			expect(farmStore.transcript('TASK-001')).toEqual(['working']);

			// Anything else still refreshes.
			apiSnapshot.mockResolvedValueOnce(sampleSnapshot([]));
			eventHandler?.({
				payload: { kind: 'taskStatusChanged', task: 'TASK-001', status: 'done', note: null }
			});
			await vi.advanceTimersByTimeAsync(1000);
			expect(apiSnapshot).toHaveBeenCalledOnce();
		} finally {
			vi.useRealTimers();
		}
	});

	it('reads leftover worktrees on open, and not on every refresh', async () => {
		// TASK-030: the scan runs `git worktree list`, and a refresh happens
		// after every burst of events.
		apiOpen.mockResolvedValueOnce(sampleSnapshot([]));
		vi.mocked(api.stale).mockResolvedValueOnce([
			{ task: 'TASK-009', path: '/repo/.spagitty/farm/task-009', branch: 'spagitty-farm/x' }
		]);
		await farmStore.open('/repo');
		expect(farmStore.stale).toHaveLength(1);

		vi.mocked(api.stale).mockClear();
		apiSnapshot.mockResolvedValueOnce(sampleSnapshot([]));
		await farmStore.refresh();
		expect(api.stale).not.toHaveBeenCalled();
		// And what was read on open is still on screen.
		expect(farmStore.stale).toHaveLength(1);
	});

	it('answers why a queued task is not running, from the backend', async () => {
		// FEAT-075. The reason depends on path leases and agent availability,
		// which only the scheduler holds — so the screen asks rather than
		// guessing.
		const snapshot = sampleSnapshot([sampleTask('TASK-001', { status: 'ready' })]);
		snapshot.waiting = { 'TASK-001': 'TASK-002 is working on the same files.' };
		apiOpen.mockResolvedValueOnce(snapshot);
		await farmStore.open('/repo');

		expect(farmStore.waitingFor('TASK-001')).toBe('TASK-002 is working on the same files.');
		expect(farmStore.waitingFor('TASK-999')).toBeNull();
	});

	it('lists the drafts a planner proposed', async () => {
		apiOpen.mockResolvedValueOnce(
			sampleSnapshot([
				sampleTask('TASK-001', { status: 'draft' }),
				sampleTask('TASK-002', { status: 'ready' }),
				sampleTask('TASK-003', { status: 'draft' })
			])
		);
		await farmStore.open('/repo');

		expect(farmStore.drafts.map((task) => task.id)).toEqual(['TASK-001', 'TASK-003']);
	});

	it('reads the task list as an outline, parents then children', async () => {
		// FEAT-076. The list renders as rows, so what it needs is a depth per
		// row rather than a tree.
		apiOpen.mockResolvedValueOnce(
			sampleSnapshot([
				sampleTask('TASK-001'),
				sampleTask('TASK-002'),
				sampleTask('TASK-003', { parent: 'TASK-001', status: 'done' }),
				sampleTask('TASK-004', { parent: 'TASK-001' })
			])
		);
		await farmStore.open('/repo');

		expect(farmStore.outline.map((row) => [row.task.id, row.depth])).toEqual([
			['TASK-001', 0],
			['TASK-003', 1],
			['TASK-004', 1],
			['TASK-002', 0]
		]);
		// The heading counts what is under it.
		expect(farmStore.outline[0]).toMatchObject({ done: 1, total: 2 });
		// A task nothing was cut out of has no fraction to show.
		expect(farmStore.outline[3]).toMatchObject({ done: 0, total: 0 });
	});

	it('shows a task whose heading was deleted rather than losing it', async () => {
		apiOpen.mockResolvedValueOnce(
			sampleSnapshot([sampleTask('TASK-009', { parent: 'TASK-GONE' })])
		);
		await farmStore.open('/repo');

		expect(farmStore.outline.map((row) => [row.task.id, row.depth])).toEqual([['TASK-009', 0]]);
	});

	it('is already listening while the backend is still answering', async () => {
		// BUG-022. `farm_open` starts agent detection on a thread and reports
		// the result as an event a few hundred milliseconds later. Subscribing
		// after the command returned left a window where that event — the one
		// that decides whether the farm has any agents — was emitted with
		// nobody listening, and the screen said "Not installed" about agents
		// sitting on PATH until something unrelated caused a refresh.
		await farmStore.stop();
		eventHandler = null;

		let listeningWhenAsked = false;
		apiOpen.mockImplementationOnce(async () => {
			listeningWhenAsked = eventHandler !== null;
			return sampleSnapshot([]);
		});

		await farmStore.open('/repo');

		expect(
			listeningWhenAsked,
			'the backend was asked to do work before anyone was listening for the answer'
		).toBe(true);
	});

	it('stop cleans up event listener and clears error', async () => {
		apiOpen.mockResolvedValueOnce(sampleSnapshot([]));
		await farmStore.open('/repo');

		farmStore.fail('Something went wrong');
		expect(farmStore.error).toBe('Something went wrong');

		farmStore.clearError();
		expect(farmStore.error).toBeNull();

		await farmStore.stop();
		expect(unlistenMock).toHaveBeenCalled();
	});
});

describe('priming a farm for the rail dot (FEAT-109)', () => {
	const apiExists = vi.mocked(api.exists);

	beforeEach(() => {
		farmStore.reset();
		apiOpen.mockReset();
		apiExists.mockReset();
	});

	it('opens nothing in a repository with no farm', async () => {
		// Opening writes the agent registry into the repository.
		apiExists.mockResolvedValue(false);
		await farmStore.prime('/repos/plain');
		expect(apiOpen).not.toHaveBeenCalled();
		expect(farmStore.path).toBeNull();
	});

	it('opens a repository that has a farm, and says which one it holds', async () => {
		apiExists.mockResolvedValue(true);
		apiOpen.mockResolvedValue(sampleSnapshot([sampleTask('T1')]));
		await farmStore.prime('/repos/farmed');
		expect(apiOpen).toHaveBeenCalledWith('/repos/farmed');
		expect(farmStore.path).toBe('/repos/farmed');
	});

	it('never replaces a farm with a run in flight', async () => {
		// The backend holds one farm; opening another stops a planner mid-plan.
		apiExists.mockResolvedValue(true);
		const busy = sampleSnapshot([sampleTask('T1', { status: 'running' })]);
		busy.runs = [
			{
				id: 'R1',
				task: 'planning',
				agent: 'claude-1',
				phase: 'planning',
				outcome: { state: 'running' },
				command: [],
				startedMs: 1,
				endedMs: null,
				logFile: null,
				lastOutputMs: null
			}
		];
		apiOpen.mockResolvedValue(busy);
		await farmStore.prime('/repos/a');
		apiOpen.mockClear();
		await farmStore.prime('/repos/b');
		expect(apiOpen).not.toHaveBeenCalled();
		expect(farmStore.path).toBe('/repos/a');
	});

	it('treats a failed look as no farm', async () => {
		apiExists.mockRejectedValue(new Error('gone'));
		await farmStore.prime('/repos/broken');
		expect(apiOpen).not.toHaveBeenCalled();
	});
});
