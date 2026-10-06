// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The words the Farm screen uses (FEAT-073).
 *
 * Pure functions, in one file, for the same reason the icon set is data: a
 * label written inline in three components is three labels the first time one
 * of them is reworded. It is also the whole of what the screen's tests assert
 * about copy, which keeps the component tests about behaviour.
 *
 * The tone follows TASK-007: say what is true, not what to do. "Nothing was
 * checked" rather than "Configure verification commands to check this task".
 */

import type {
	AgentAvailability,
	AgentRun,
	AgentProvider,
	Autonomy,
	FarmEvent,
	FarmStatus,
	TaskKind,
	TaskOrigin,
	TaskStatus,
	Verification
} from './types';

export const PROVIDER_LABELS: Record<AgentProvider, string> = {
	claudeCode: 'Claude Code',
	codex: 'Codex',
	cursor: 'Cursor',
	ohMyPi: 'Oh My Pi',
	custom: 'Custom'
};

export const TASK_STATUS_LABELS: Record<TaskStatus, string> = {
	draft: 'Draft',
	ready: 'Ready',
	assigned: 'Assigned',
	running: 'Running',
	waiting: 'Waiting',
	blocked: 'Blocked',
	review: 'Review',
	verification: 'Verification',
	done: 'Done',
	failed: 'Failed',
	cancelled: 'Cancelled'
};

export const FARM_STATUS_LABELS: Record<FarmStatus, string> = {
	idle: 'Idle',
	planning: 'Planning',
	running: 'Running',
	paused: 'Paused',
	blocked: 'Blocked',
	reviewing: 'Reviewing',
	completed: 'Completed',
	failed: 'Failed',
	cancelled: 'Cancelled'
};

export const TASK_KIND_LABELS: Record<TaskKind, string> = {
	architecture: 'Architecture',
	backend: 'Backend',
	frontend: 'Frontend',
	testing: 'Testing',
	documentation: 'Documentation',
	research: 'Research',
	review: 'Review',
	integration: 'Integration',
	general: 'General'
};

/**
 * The autonomy levels, with what each one actually does.
 *
 * The description matters more than the name: "Semi-automatic" tells nobody
 * where the human is, and the whole point of the setting is to say where the
 * human is.
 */
export const AUTONOMY_LEVELS: { id: Autonomy; label: string; detail: string }[] = [
	{ id: 'manual', label: 'Manual', detail: 'Tasks are organised here. Nothing runs by itself.' },
	{ id: 'assisted', label: 'Assisted', detail: 'Agents run. You approve every task.' },
	{ id: 'semiAuto', label: 'Semi-automatic', detail: 'Agents run and review each other. You approve merges.' },
	{ id: 'auto', label: 'Automatic', detail: 'Verified and reviewed tasks merge themselves.' },
	{ id: 'yolo', label: 'Unattended', detail: 'Everything, including retrying failures, without asking.' }
];

/** How a task's status should read on a chip. */
export function taskStatusLabel(status: TaskStatus): string {
	return TASK_STATUS_LABELS[status];
}

/**
 * The tone a status chip takes.
 *
 * Four, not eleven: a chip per status would be a rainbow, and the reader only
 * needs to know whether something is finished, moving, stuck, or waiting.
 */
export type Tone = 'done' | 'active' | 'stuck' | 'idle';

export function tone(status: TaskStatus): Tone {
	switch (status) {
		case 'done':
			return 'done';
		case 'running':
		case 'verification':
		case 'review':
		case 'assigned':
			return 'active';
		case 'failed':
		case 'blocked':
			return 'stuck';
		default:
			return 'idle';
	}
}

/** True while something is happening to this task without anyone pressing anything. */
export function isMoving(status: TaskStatus): boolean {
	return status === 'running' || status === 'verification' || status === 'assigned';
}

/** What an availability means, in one line. */
export function availabilityLabel(availability: AgentAvailability): string {
	switch (availability.state) {
		case 'available':
			return availability.version || 'Detected';
		case 'broken':
			return `Found, but it did not run: ${availability.reason}`;
		case 'missing':
			return 'Not installed';
	}
}

/** One line for an event, for the activity list. */
export function eventLine(event: FarmEvent): string {
	switch (event.kind) {
		case 'farmStatusChanged':
			return `The farm is ${FARM_STATUS_LABELS[event.status].toLowerCase()}`;
		case 'taskCreated':
			return `${event.task} created — ${event.title}`;
		case 'taskStatusChanged':
			return event.note
				? `${event.task} is ${TASK_STATUS_LABELS[event.status].toLowerCase()} — ${event.note}`
				: `${event.task} is ${TASK_STATUS_LABELS[event.status].toLowerCase()}`;
		case 'taskAssigned':
			return `${event.task} assigned to ${event.agent}`;
		case 'agentStarted':
			return `${event.agent} started on ${event.task}`;
		case 'agentOutput':
			return event.line;
		case 'agentStopped':
			return event.ok
				? `The agent finished ${event.task}`
				: `The agent stopped on ${event.task}${event.reason ? ` — ${event.reason}` : ''}`;
		case 'verificationStarted':
			return `${event.task}: running ${event.command}`;
		case 'verificationFinished':
			return `${event.task}: ${event.command} ${event.passed ? 'passed' : 'failed'}`;
		case 'reviewRequested':
			return `${event.task} sent to ${event.reviewer} for review`;
		case 'reviewCompleted':
			return `${event.reviewer} ${event.approved ? 'approved' : 'asked for changes on'} ${event.task}`;
		case 'supplementalPolicyChanged':
			return `Supplemental reviews: ${event.mode} (changed by ${event.actor})`;
		case 'supplementalReview':
			return `${event.task}: ${event.summary}`;
		case 'mergeRequested':
			return `Merging ${event.branch}`;
		case 'mergeCompleted':
			return event.ok
				? `Merged ${event.branch}`
				: `${event.branch} did not merge${event.error ? ` — ${event.error}` : ''}`;
		case 'workspaceChanged':
			return event.created
				? `${event.task}: worktree created`
				: `${event.task}: worktree removed`;
		case 'taskProposed':
			return `${event.from} proposed: ${event.title}`;
		case 'failed':
			return event.message;
	}
}

/**
 * What a verification result says, in one line.
 *
 * `unverified` is deliberately not phrased as a pass. A green tick for "we
 * checked nothing" is the one lie this whole feature rests on not telling.
 */
export function verificationLine(verification: Verification | null): string {
	if (!verification) return 'Not run';
	if (verification.unverified) return 'Nothing was checked — this farm has no verification commands';
	if (verification.passed) {
		const count = verification.results.length;
		return `${count} ${count === 1 ? 'check' : 'checks'} passed`;
	}
	const failed = verification.results.filter((result) => !result.passed);
	return `Failed: ${failed.map((result) => result.command).join(', ')}`;
}

/**
 * Why a task cannot run yet, or nothing.
 *
 * Read by the task row, so a queue of tasks that are not moving explains itself
 * rather than looking stalled.
 */
export function waitingOn(
	task: { dependsOn: string[]; allowedPaths: string[] },
	byId: Map<string, { status: TaskStatus }>
): string | null {
	const unmet = task.dependsOn.filter((id) => byId.get(id)?.status !== 'done');
	if (unmet.length > 0) {
		return `Waiting for ${unmet.join(', ')}`;
	}
	if (task.allowedPaths.length === 0) {
		// The consequence of the safe reading of an unanswered question, and
		// worth saying: it is why a farm with three ready tasks runs one.
		return 'No paths declared, so this runs on its own';
	}
	return null;
}

/**
 * Who asked for a task, in one line (FEAT-078).
 *
 * Written as a fact rather than as a warning. A proposed task is not suspect —
 * it is often the best idea in the plan — but it was nobody's decision until
 * somebody accepts it, and the sentence says so plainly.
 */
export function originLine(origin: TaskOrigin): string {
	switch (origin.kind) {
		case 'person':
			return 'You added this.';
		case 'planned':
			return `${origin.agent} cut this out of the goal.`;
		case 'subtask':
			return `${origin.agent} cut this out of ${origin.parent}.`;
		case 'proposed':
			return origin.agent
				? `${origin.agent} proposed this while working on ${origin.from}. Nobody asked for it.`
				: `Proposed while working on ${origin.from}. Nobody asked for it.`;
	}
}

/**
 * The mark a row carries for who asked.
 *
 * Two characters and no colour: a person's own work is unmarked, because most
 * rows in most farms are theirs and a mark on everything marks nothing.
 */
export function originMark(origin: TaskOrigin): string | null {
	return origin.kind === 'person' ? null : '⌁';
}

/**
 * How long a run may say nothing before the screen mentions it (FEAT-077).
 *
 * Six minutes. Long enough that a model reading a large repository is not
 * flagged for thinking, short enough that a run which died is noticed while the
 * person is still at the machine. Nothing is stopped on the strength of it: a
 * quiet run is flagged, and stopping stays the reader's decision.
 */
export const QUIET_AFTER_MS = 6 * 60 * 1000;

/**
 * What to say about a run that has gone quiet, or nothing.
 *
 * `null` for a run that is talking, a run that is finished, and a run that has
 * been quiet for less than [`QUIET_AFTER_MS`] — the overwhelming majority.
 */
export function quietLine(run: AgentRun | null, now: number): string | null {
	if (!run || run.outcome.state !== 'running') return null;
	const since = run.lastOutputMs ?? run.startedMs;
	const quiet = now - since;
	if (quiet < QUIET_AFTER_MS) return null;
	return `No output for ${duration(quiet)}.`;
}

/** A duration, for a run row. */
export function duration(ms: number | null | undefined): string {
	if (ms === null || ms === undefined) return '';
	if (ms < 1000) return `${ms}ms`;
	const seconds = Math.round(ms / 1000);
	if (seconds < 60) return `${seconds}s`;
	const minutes = Math.floor(seconds / 60);
	return `${minutes}m ${seconds % 60}s`;
}
