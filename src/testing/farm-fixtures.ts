// SPDX-License-Identifier: GPL-3.0-or-later
import type { Farm, FarmSnapshot, Task } from '$lib/farm/types';
export function sampleTask(id: string, overrides: Partial<Task> = {}): Task {
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

export function sampleFarm(tasks: Task[] = []): Farm {
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

export function sampleSnapshot(tasks: Task[] = []): FarmSnapshot {
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

