// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it, vi } from 'vitest';
import { QUIET_AFTER_MS, quietLine } from './describe';
import { actorKind } from './delight';
import type { AgentRun, Task } from './types';

/**
 * A farm worth watching (FEAT-077).
 *
 * The three promises: the ring says where the work is without being read, the
 * strip says who is doing what, and a run that has gone quiet says so — while
 * never being stopped on the strength of it.
 */

const NOW = 1_700_000_000_000;

function run(overrides: Partial<AgentRun> = {}): AgentRun {
	return {
		id: 'run-1',
		task: 'TASK-0001',
		agent: 'claude',
		phase: 'implementation',
		outcome: { state: 'running' },
		command: ['claude -p'],
		startedMs: NOW - 60_000,
		endedMs: null,
		logFile: null,
		lastOutputMs: NOW - 1_000,
		...overrides
	};
}

describe('when a run has gone quiet', () => {
	it('says nothing about a run that is talking', () => {
		expect(quietLine(run(), NOW)).toBeNull();
	});

	it('says nothing about a finished run, however long ago it spoke', () => {
		// A finished run is not quiet, it is finished.
		expect(
			quietLine(run({ outcome: { state: 'completed', exitCode: 0 }, lastOutputMs: 0 }), NOW)
		).toBeNull();
	});

	it('measures from when it started if it has never said anything', () => {
		// The case this exists for: an agent that produced no output at all
		// looks exactly like one that died on its first second.
		const line = quietLine(run({ lastOutputMs: null, startedMs: NOW - QUIET_AFTER_MS - 1 }), NOW);
		expect(line).toContain('No output for');
	});

	it('holds its tongue until the threshold', () => {
		expect(quietLine(run({ lastOutputMs: NOW - QUIET_AFTER_MS + 1_000 }), NOW)).toBeNull();
		expect(quietLine(run({ lastOutputMs: NOW - QUIET_AFTER_MS - 1_000 }), NOW)).not.toBeNull();
	});
});

describe('the delight seam', () => {
	it('recognises the agents the layer draws, and calls the rest agents', () => {
		expect(actorKind('claude')).toBe('claude');
		expect(actorKind('codex')).toBe('codex');
		expect(actorKind('my-gpt-runner')).toBe('gpt');
		// A custom agent is an agent. Not being able to name its model is not a
		// reason to refuse it a record.
		expect(actorKind('some-inhouse-thing')).toBe('agent');
	});
});
