// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it, vi } from 'vitest';

// The store reaches the backend through `./api` and the Tauri event channel.
// Neither exists here, and neither is what these tests are about.
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(() => Promise.resolve(vi.fn()))
}));
vi.mock('./api', () => ({
	open: vi.fn(),
	snapshot: vi.fn(),
	stale: vi.fn(() => Promise.resolve([])),
	failure: vi.fn((error: unknown) => ({ kind: 'testError', message: String(error) }))
}));

import * as api from './api';
import { farmStore, PLANNING_TASK } from './store.svelte';
import type { AgentRun, FarmSnapshot } from './types';

/**
 * A planning run, while it is running (BUG-021).
 *
 * Two promises, and they are the whole of the fix from the screen's side: a
 * planning run in flight is visible, and it can be stopped. What the planner
 * says is narrated in the backend, so the assertions here are about the card
 * showing the *latest* thing rather than about the words themselves.
 */

function planningRun(overrides: Partial<AgentRun> = {}): AgentRun {
	return {
		id: 'plan-1',
		task: PLANNING_TASK,
		agent: 'claude',
		phase: 'planning',
		outcome: { state: 'running' },
		command: ['claude -p'],
		startedMs: 1_000,
		endedMs: null,
		logFile: null,
		lastOutputMs: null,
		...overrides
	};
}

function snapshot(runs: AgentRun[]): FarmSnapshot {
	return {
		farm: null,
		agents: [],
		undetected: [],
		events: [],
		runs,
		policy: { sources: [], text: '' },
		scoreboard: [],
		waiting: {}
	};
}

describe('the store, about planning', () => {
	it('keeps the planner’s own transcript, which belongs to no task', () => {
		farmStore.reset();
		farmStore.absorb({
			atMs: 1_700_000_000_000,
			kind: 'agentOutput',
			run: 'plan-1',
			task: PLANNING_TASK,
			line: '· Read src/auth.rs'
		});

		expect(farmStore.planning).toEqual(['· Read src/auth.rs']);
		// And it is not in the activity list: transcripts never were.
		expect(farmStore.activity).toHaveLength(0);
	});

	it('finds the planning run in flight, and lets go of it when it ends', async () => {
		farmStore.reset();
		vi.mocked(api.open).mockResolvedValueOnce(snapshot([planningRun()]));
		await farmStore.open('/repo');
		expect(farmStore.planningRun?.id).toBe('plan-1');

		vi.mocked(api.snapshot).mockResolvedValueOnce(
			snapshot([planningRun({ outcome: { state: 'completed', exitCode: 0 } })])
		);
		await farmStore.refresh();
		expect(farmStore.planningRun).toBeNull();
	});

	it('does not mistake a task’s run for a planning run', async () => {
		farmStore.reset();
		vi.mocked(api.open).mockResolvedValueOnce(
			snapshot([planningRun({ id: 'run-9', task: 'T-1', phase: 'implementation' })])
		);
		await farmStore.open('/repo');

		expect(farmStore.planningRun).toBeNull();
	});
});
