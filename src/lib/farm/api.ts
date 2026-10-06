// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Typed wrappers around the farm's Tauri commands (FEAT-073).
 *
 * The same rule as `$lib/api.ts`: this is the only farm module that calls
 * `invoke`, so a command rename is a one-file change. It is separate from
 * `api.ts` because the farm is a subsystem with its own backend module, and one
 * file listing every command in the product would be the longest file in the
 * frontend for no benefit.
 */

import { invoke } from '@tauri-apps/api/core';
import type {
	AgentDefinition,
	AgentStatus,
	Farm,
	FarmFailure,
	FarmSettings,
	FarmSnapshot,
	RecordedEvent,
	StaleWorkspace,
	Task,
	TaskDetail,
	TaskDraft
} from './types';

/**
 * Point the farm at a repository.
 *
 * Returns immediately with whatever was saved; agent detection runs on a
 * thread and announces itself with an event.
 */
export function open(path: string): Promise<FarmSnapshot> {
	return invoke('farm_open', { path });
}

export function close(): Promise<void> {
	return invoke('farm_close');
}

export function snapshot(): Promise<FarmSnapshot> {
	return invoke('farm_snapshot');
}

export function detectAgents(): Promise<AgentStatus[]> {
	return invoke('farm_detect_agents');
}

export function saveAgent(agent: AgentDefinition): Promise<void> {
	return invoke('farm_save_agent', { agent });
}

export function removeAgent(id: string): Promise<void> {
	return invoke('farm_remove_agent', { id });
}

export function create(title: string, description: string): Promise<Farm> {
	return invoke('farm_create', { title, description });
}

export function configure(settings: FarmSettings): Promise<Farm> {
	return invoke('farm_configure', { settings });
}

export function start(): Promise<void> {
	return invoke('farm_start');
}

export function pause(): Promise<void> {
	return invoke('farm_pause');
}

export function cancel(): Promise<void> {
	return invoke('farm_cancel');
}

/** Write the starter `AGENTS.md`. Rejects if one already exists. */
export function writePolicy(): Promise<string> {
	return invoke('farm_write_policy');
}

export function addTask(draft: TaskDraft): Promise<Task> {
	return invoke('farm_add_task', { draft });
}

export function editTask(id: string, draft: TaskDraft): Promise<Task> {
	return invoke('farm_edit_task', { id, draft });
}

export function deleteTask(id: string): Promise<void> {
	return invoke('farm_delete_task', { id });
}

/** Move a draft into the plan, and let the scheduler look at it. */
export function readyTask(id: string): Promise<void> {
	return invoke('farm_ready_task', { id });
}

/**
 * Accept a plan: move several drafts into it at once.
 *
 * One call rather than one per task — accepting a plan is one decision, and
 * eight calls would be eight writes and eight chances to land half of it.
 */
export function readyTasks(ids: string[]): Promise<void> {
	return invoke('farm_ready_tasks', { ids });
}

/** Throw several drafts away. */
export function discardTasks(ids: string[]): Promise<void> {
	return invoke('farm_discard_tasks', { ids });
}

export function assignTask(id: string, agent: string): Promise<void> {
	return invoke('farm_assign_task', { id, agent });
}

export function cancelTask(id: string): Promise<void> {
	return invoke('farm_cancel_task', { id });
}

export function retryTask(id: string): Promise<void> {
	return invoke('farm_retry_task', { id });
}

/** Start one task now. Returns as soon as the agent is running. */
export function runTask(id: string, agent: string | null): Promise<void> {
	return invoke('farm_run_task', { id, agent });
}

export function taskDetail(id: string): Promise<TaskDetail> {
	return invoke('farm_task_detail', { id });
}

export function transcript(run: string, task: string): Promise<string> {
	return invoke('farm_transcript', { run, task });
}

export function mergeTask(id: string): Promise<void> {
	return invoke('farm_merge_task', { id });
}

export function reviewTask(id: string): Promise<void> {
	return invoke('farm_review_task', { id });
}

export function verifyTask(id: string): Promise<void> {
	return invoke('farm_verify_task', { id });
}

/** Ask an agent to break the goal into tasks. Resolves with the run id. */
export function plan(agent: string | null): Promise<string> {
	return invoke('farm_plan', { agent });
}

/**
 * Ask an agent to break one task into smaller ones.
 *
 * The same planning run pointed at a task. Its children arrive as drafts, so
 * the plan-review band asks before any of them runs.
 */
export function decompose(id: string, agent: string | null): Promise<string> {
	return invoke('farm_decompose', { id, agent });
}

/**
 * Stop a planning run.
 *
 * Not `cancel()`, which stops the whole farm. Changing your mind about a
 * decomposition is not changing your mind about the work.
 */
export function cancelPlan(): Promise<void> {
	return invoke('farm_cancel_plan');
}

/**
 * Worktrees left behind by tasks no farm claims.
 *
 * Its own call rather than part of the snapshot: answering it runs `git
 * worktree list`, and the snapshot is taken after every burst of events.
 */
export function stale(): Promise<StaleWorkspace[]> {
	return invoke('farm_stale');
}

/** More activity than a snapshot carries, for a reader scrolling back. */
export function events(limit?: number): Promise<RecordedEvent[]> {
	return invoke('farm_events', { limit: limit ?? null });
}

export function sweep(): Promise<StaleWorkspace[]> {
	return invoke('farm_sweep');
}

/**
 * Read a rejected command's `{ kind, message }`.
 *
 * Tauri rejects with whatever the command returned, and the farm returns a
 * struct rather than a string so the interface can branch on the *kind* — an
 * unavailable agent gets an install hint, a contended path gets "wait or
 * reassign". Anything that is not that shape is still shown, because an
 * unexpected failure with no message is worse than an ugly one.
 */
export function failure(error: unknown): FarmFailure {
	if (typeof error === 'object' && error !== null && 'kind' in error && 'message' in error) {
		const candidate = error as { kind: unknown; message: unknown };
		if (typeof candidate.kind === 'string' && typeof candidate.message === 'string') {
			return { kind: candidate.kind, message: candidate.message };
		}
	}
	return { kind: 'unknown', message: String(error) };
}

export function reviewSupplemental(id: string): Promise<import('./types').SupplementalEvidence> { return invoke('farm_review_supplemental', { id }); }
