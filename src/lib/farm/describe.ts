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
	AgentCapability,
	AgentDefinition,
	AgentRun,
	AgentProvider,
	AgentStatus,
	Autonomy,
	Farm,
	FarmEvent,
	FarmStatus,
	RecordedEvent,
	Task,
	TaskDetail,
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
export const QUIET_AFTER_MS = 3 * 60 * 1000;

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

/** Whether a live run has said nothing for [`QUIET_AFTER_MS`] or longer. */
export function isQuiet(run: AgentRun | null | undefined, now: number): boolean {
	return !!run && run.outcome.state === 'running' && quietLine(run, now) !== null;
}

/**
 * A live run in the board's words: "6 min · spoke 8 s ago", or, once it has
 * gone quiet, "11 min · quiet for 4 min".
 */
export function runLine(run: AgentRun, now: number): string {
	const since = run.lastOutputMs ?? run.startedMs;
	const spoke = isQuiet(run, now)
		? `quiet for ${relativeTime(since, now)}`
		: `spoke ${relativeTime(since, now)} ago`;
	return `${relativeTime(run.startedMs, now)} · ${spoke}`;
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

/** Farm's journey vocabulary and presentation rules (FEAT-108). */
export const PHASES = ['Crew', 'Goal', 'Plan', 'Build', 'Wrap up'] as const;
export const STEPS = ['Queued', 'Worktree', 'Working', 'Checks', 'Review', 'Landed'] as const;
export const PANES = ['Board', 'Crew', 'Activity'] as const;
export const TASK_TABS = ['Output', 'Changes', 'Checks', 'Review', 'Hand-off'] as const;
export const FARM_COPY = {
	setupTitle: 'Put a crew of agents on one goal',
	setupBrief:
		'One agent plans the work. Each task then gets its own agent and its own worktree, your checks run on it, a second agent reviews it, and it lands on your branch. You choose where you come in.',
	paused: 'Paused. Nothing new starts; the agents already working are left to finish.',
	ready: 'Ready to land. It needs your yes.',
	crew: 'Your crew',
	goal: 'The goal',
	rules: 'Rules',
	needsYou: 'Needs you',
	upNext: 'Up next',
	working: 'Working',
	checking: 'Checking',
	landed: 'Landed',
	upNextDetail: 'Waits for a free agent or an earlier task',
	workingDetail: 'Each agent in its own worktree',
	checkingDetail: 'Your commands run, then another agent reviews',
	firstFree: 'First free agent',
	planReady: 'The plan is ready',
	nothingRuns: 'Nothing runs until you accept the plan.',
	manualTasks: "I'll write the tasks myself",
	lookAgain: 'Look again',
	addCli: 'Add a CLI agent',
	stopPlanning: 'Stop planning',
	reviewPlan: 'Review the plan',
	startBuilding: 'Start building',
	discard: 'Discard',
	planAgain: 'Plan again',
	changeRules: 'Change the rules',
	whoDoesWhat: 'Who does what',
	whenStarts: 'When it starts',
	worthLook: 'Worth a look',
	free: 'Free',
	lately: 'Lately',
	allActivity: 'All activity',
	landIt: 'Land it',
	seeChanges: 'See the changes',
	openTask: 'Open the task',
	pause: 'Pause',
	resume: 'Resume',
	addTask: 'Task',
	autonomy: 'Autonomy',
	checks: 'Checks',
	maxAttempts: 'Tries per task',
	moreRules: 'More rules',
	saveRules: 'Save rules',
	onCrew: 'On the crew',
	crewFound: 'How the crew is found',
	crewFoundDetail:
		"Spagitty looks on PATH for command-line agents and runs each once to read its version. Only agents that run headless from a terminal can join; editor plug-ins can't.",
	assignment: 'Who gets which task',
	assignmentDetail:
		"A task goes to its suggested agent. If that one is busy, the first free agent whose skills match the task's kind takes it. An agent never reviews its own work.",
	anotherAgent: 'Another agent',
	cliDetail: 'Any CLI that takes a prompt as an argument or on stdin and exits when it is done.',
	arguments: 'Arguments',
	landedStat: 'landed',
	sentBack: 'sent back',
	failedChecks: 'failed checks',
	typical: 'a task, typically',
	whatHappened: 'What happened',
	newestFirst: 'Newest first',
	all: 'All',
	failures: 'Failures',
	whatLanded: 'What landed',
	raised: 'The agents raised',
	raisedDetail: 'from their hand-offs, not acted on yet',
	whoDidWhat: 'Who did what',
	tidyUp: 'Tidy up',
	removeWorktrees: 'Remove worktrees and branches',
	newGoal: 'Start a new goal',
	answer: 'Answer',
	makeTask: 'Make it a task',
	startWith: 'Start a farm with it',
	sendBack: 'Send it back',
	giveUp: 'Give up',
	edit: 'Edit the task',
	stop: 'Stop',
	openWorktree: 'Open the worktree',
	reassign: 'Reassign',
	brief: 'The brief',
	doneWhen: 'Done when',
	mayTouch: 'May touch',
	needs: 'Needs',
	askedBy: 'Asked by',
	branch: 'Branch',
	worktree: 'Worktree',
	fullLog: 'Full log',
	openBranch: 'Open the branch in Graph',
	noActivity: 'Activity arrives when the farm moves.',
	noOutput: 'Output appears when the agent starts.',
	noChanges: 'Changed files appear after the agent hands off.',
	noChecks: 'Checks run once the agent finishes.',
	noReview: 'Another agent reviews it once the checks pass.',
	noHandoff: 'The agent leaves a hand-off when it finishes.',
	noTasks: 'Add a task or ask an agent to plan the goal.',
	noRaised: 'No questions, risks or follow-up tasks were raised.',
	busyDisabled: 'It finishes its current run, then takes no new work.',
	policyFound: 'AGENTS.md found',
	policyMissing: 'No AGENTS.md yet',
	policyWrite: 'Write the starter AGENTS.md',
	goalTitle: 'A short title',
	goalNotes: 'Notes for the planner',
	checksHint: 'One command per line',
	permission: 'Permissions',
	leftovers: 'Leftover worktrees',
	supplemental: 'CodeRabbit',
	quiet: 'quiet',
	cancelled: 'Show cancelled',
	planningDoing: 'What the planner is doing',
	givenBrief: 'The brief it was given',
	plannerReadOnly: 'Read-only. It changes no files.',
	plannerGiven: 'It was given',
	plannerGivenDetail:
		"The goal, your notes, AGENTS.md, the repository's layout, and the agents on the crew.",
	plannerTaskSays: 'Each task it proposes says',
	plannerTaskParts: [
		'what to do, and when it is done',
		'which files it may touch',
		'what it waits for',
		'which agent suits it'
	],
	plannerThen: 'Then',
	plannerThenDetail: 'You review the plan. Nothing runs until you accept it.',
	plannerReading: 'reading the repository and the goal',
	setupFlow: [
		'Goal',
		'Plan',
		'Task in its own worktree',
		'Your checks',
		'Another agent reviews',
		'Lands on main'
	],
	crewDetail: 'Command-line agents found on PATH. Only CLI agents can join a farm.',
	crewAnother: 'Another CLI agent? Give Spagitty its command and how it takes a prompt.',
	goalDetail:
		'One outcome. The planner splits it into tasks; you can edit every one before anything runs.',
	goalQuestion: 'What should be true when the farm is done?',
	goalKnow: 'Anything the agents should know',
	rulesDetail:
		'Where you come in, what counts as done, and how many agents work at once. You can change these while it runs.',
	howMuch: 'How much it does on its own',
	youComeIn: 'You come in:',
	mustPass: 'Checks every task must pass',
	addCommand: 'Add a command',
	atOnce: 'Agents at once',
	triesBefore: 'Tries before a task needs you',
	policyAttached: 'AGENTS.md is attached to every prompt',
	planningBar:
		'Planning reads the goal and the repository, then proposes tasks. Nothing runs until you accept them.',
	onTheCrew: 'Ready · on the crew',
	leftOut: 'Ready · left out',
	noTasksYet: 'no tasks yet',
	cycle: "Can't be ordered",
	missingDependency: 'The plan needs tasks that are missing.',
	close: 'Close',
	save: 'Save',
	cliName: 'Name',
	cliExecutable: 'Executable',
	input: 'Prompt input'
} as const;

export function agentName(id: string | null | undefined, agents: AgentStatus[]): string {
	return (
		agents.find((a) => a.definition.id === id)?.definition.displayName ?? id ?? FARM_COPY.firstFree
	);
}
export function agentColour(agent: AgentDefinition | undefined): string {
	return `var(--${({ claudeCode: 'lane-1', codex: 'lane-5', cursor: 'lane-4', ohMyPi: 'lane-3', custom: 'muted' } as const)[agent?.provider ?? 'custom']})`;
}
export function agentMonogram(agent: AgentDefinition | undefined): string {
	if (!agent) return '—';
	return (
		{
			claudeCode: 'CC',
			codex: 'CX',
			cursor: 'CU',
			ohMyPi: 'PI',
			custom: agent.displayName.slice(0, 2).toUpperCase()
		} as const
	)[agent.provider];
}
/** "You come in:" — the handoff's table, one line per autonomy level. */
export function whereYouComeIn(autonomy: Autonomy): string {
	return {
		manual: 'you start every task and every merge yourself.',
		assisted: 'after each task, before it is reviewed or merged.',
		semiAuto: 'only to approve merges, and when a task is stuck.',
		auto: 'only when a task is stuck after its tries.',
		yolo: 'at the end. Nothing waits for you.'
	}[autonomy];
}

/** Capabilities as they read inside a sentence, most telling first. */
const STRENGTH_WORDS: [AgentCapability, string][] = [
	['frontend', 'frontend'],
	['backend', 'backend'],
	['review', 'review'],
	['research', 'research'],
	['testing', 'testing'],
	['documentation', 'docs'],
	['longContext', 'long context'],
	['vision', 'vision'],
	['toolUse', 'tool use']
];

/**
 * What an agent is good at, in one line: "Plans well · backend, review".
 *
 * Coding is left out because every agent on a farm codes; saying so on each
 * tile would only push the useful words off the end.
 */
export function strengths(agent: AgentDefinition): string {
	const rest = STRENGTH_WORDS.filter(([id]) => agent.capabilities.includes(id))
		.slice(0, 3)
		.map(([, word]) => word)
		.join(', ');
	if (agent.capabilities.includes('planning')) return rest ? `Plans well · ${rest}` : 'Plans well';
	return rest ? rest[0].toUpperCase() + rest.slice(1) : 'Coding';
}

/** Where a task sits in Building's progress bar, in the bar's order. */
export type Segment = 'landed' | 'ready' | 'stuck' | 'working' | 'checking' | 'waiting';

export const SEGMENTS: { id: Segment; label: string; colour: string }[] = [
	{ id: 'landed', label: 'landed', colour: 'var(--ok)' },
	{ id: 'ready', label: 'ready to land', colour: 'color-mix(in srgb, var(--ok) 45%, transparent)' },
	{ id: 'stuck', label: 'stuck', colour: 'var(--danger)' },
	{ id: 'working', label: 'working', colour: 'var(--accent)' },
	{ id: 'checking', label: 'checking', colour: 'var(--lane-5)' },
	{ id: 'waiting', label: 'waiting', colour: 'var(--soft)' }
];

export function segment(task: Task, detail: TaskDetail | undefined, needsYou: boolean): Segment {
	if (task.status === 'done') return 'landed';
	if (readyToLand(detail)) return 'ready';
	if (needsYou) return 'stuck';
	if (task.status === 'running') return 'working';
	if (task.status === 'verification' || task.status === 'review') return 'checking';
	return 'waiting';
}

/** "4 commits, +412 −18" — a task's branch against where it started. */
export function statsLine(stats: TaskDetail['stats']): string | null {
	if (!stats) return null;
	const added = stats.files.reduce((sum, file) => sum + file.added, 0);
	const removed = stats.files.reduce((sum, file) => sum + file.removed, 0);
	const commits = `${stats.commits} ${stats.commits === 1 ? 'commit' : 'commits'}`;
	return stats.files.length ? `${commits}, +${added} −${removed}` : commits;
}

/**
 * Why a stuck task stopped, in one line: "Failed its checks 3 of 3 times:
 * libsecret-1 was not found". The reason is the agent's own hand-off when it
 * gave one, otherwise the last line the failing check printed.
 */
export function failureLine(
	task: Task,
	detail: TaskDetail | undefined,
	maxAttempts: number
): string {
	const failed = detail?.verification?.results.find((result) => !result.passed);
	const what = failed ? 'Failed its checks' : 'Stopped';
	const times = `${task.attempts} of ${maxAttempts} ${maxAttempts === 1 ? 'time' : 'times'}`;
	const handoff = detail?.handoff?.status === 'failed' ? detail.handoff.summary : null;
	const printed = failed?.output
		.split('\n')
		.map((line) => line.trim())
		.filter(Boolean)
		.at(-1);
	const reason = handoff || task.note || printed;
	return reason ? `${what} ${times}: ${reason.replace(/\.$/, '')}` : `${what} ${times}`;
}

/** The check a task is running now, from the events: the newest one started and not finished. */
export function runningCheck(task: string, events: RecordedEvent[]): string | null {
	for (let i = events.length - 1; i >= 0; i--) {
		const event = events[i];
		if (!('task' in event) || event.task !== task) continue;
		if (event.kind === 'verificationStarted') return event.command;
		if (event.kind === 'verificationFinished') return null;
	}
	return null;
}

/**
 * Whether checks failed on what this run wrote: a failed check for its task
 * after the run started and before the task's next attempt began. Activity
 * stripes those bars red.
 */
export function checksFailedAfter(
	run: AgentRun,
	runs: AgentRun[],
	events: RecordedEvent[]
): boolean {
	if (run.phase !== 'implementation') return false;
	const next = runs
		.filter(
			(r) => r.task === run.task && r.phase === 'implementation' && r.startedMs > run.startedMs
		)
		.reduce((soonest, r) => Math.min(soonest, r.startedMs), Infinity);
	return events.some(
		(e) =>
			e.kind === 'verificationFinished' &&
			!e.passed &&
			e.task === run.task &&
			e.atMs >= run.startedMs &&
			e.atMs <= next
	);
}

/** Whether two allowed paths can touch the same file: equal, or one inside the other. */
function pathsMeet(a: string, b: string): boolean {
	const dir = (p: string) => (p.endsWith('/') ? p : `${p}/`);
	return a === b || a.startsWith(dir(b)) || b.startsWith(dir(a));
}

/**
 * Plan review's "Worth a look", generated from the plan itself: the task
 * that waits longest, and pairs of tasks whose allowed paths meet — the
 * merges most likely to need a rebase. At most four lines.
 */
export function worthALook(tasks: Task[]): string[] {
	const lines: string[] = [];
	const layers = waves(tasks);
	const last = layers.at(-1);
	if (layers.length > 1 && last && !last.unordered) {
		const longest = [...last.tasks].sort((a, b) => b.dependsOn.length - a.dependsOn.length)[0];
		const n = longest.dependsOn.length;
		lines.push(
			`${longest.id} waits for ${numberWord(n)} ${n === 1 ? 'task' : 'tasks'}, so it starts last.`
		);
	}
	for (let i = 0; i < tasks.length && lines.length < 4; i++) {
		for (let j = i + 1; j < tasks.length && lines.length < 4; j++) {
			const a = tasks[i];
			const b = tasks[j];
			const shared = a.allowedPaths.find((p) => b.allowedPaths.some((q) => pathsMeet(p, q)));
			if (shared) {
				lines.push(
					`${a.id} and ${b.id} both touch ${shared}; whichever lands second is rebased on the first.`
				);
			}
		}
	}
	return lines;
}

/** The Task screen's "Asked by": who wanted this task, by display name. */
export function askedBy(origin: TaskOrigin, agents: AgentStatus[]): string {
	switch (origin.kind) {
		case 'person':
			return 'You';
		case 'planned':
			return `Planned by ${agentName(origin.agent, agents)}`;
		case 'subtask':
			return `${agentName(origin.agent, agents)}, splitting ${origin.parent}`;
		case 'proposed':
			return origin.agent
				? `${agentName(origin.agent, agents)}, while on ${origin.from}`
				: `Proposed while on ${origin.from}`;
	}
}

/** What a transcript line is, for its colour in the Output tab. */
export type TranscriptKind = 'think' | 'edit' | 'command' | 'error';

export function transcriptKind(line: string): TranscriptKind {
	if (/\b(error|failed|panicked)\b/i.test(line)) return 'error';
	if (/^\s*\$ /.test(line)) return 'command';
	if (/^\s*(\+ |edit\b|write\b|wrote\b|patch\b)/i.test(line)) return 'edit';
	return 'think';
}

/** The stepper card's heading: one sentence for where the task is. */
export function journeyHeading(
	task: Task,
	detail: TaskDetail | undefined,
	run: AgentRun | undefined,
	agents: AgentStatus[]
): string {
	if (readyToLand(detail)) return FARM_COPY.ready;
	if (run) {
		const who = agentName(run.agent, agents);
		return run.phase === 'review' ? `${who} is reviewing it` : `${who} is writing it`;
	}
	if (task.status === 'failed' || task.status === 'blocked')
		return 'Stuck. The farm stopped trying.';
	if (task.status === 'verification') return 'Your checks are running';
	if (task.status === 'done') return 'It has landed';
	if (task.status === 'cancelled') return 'Given up';
	return 'Waiting for its turn';
}

/** "T-02", "T-02 and T-04", "T-01, T-02 and T-04". */
export function listIds(ids: string[]): string {
	return ids.length < 2 ? (ids[0] ?? '') : `${ids.slice(0, -1).join(', ')} and ${ids.at(-1)}`;
}

/**
 * What a free agent does next: "takes T-06 when T-05 lands", or, while the
 * work it handed off is being checked, that it is waiting on those checks.
 * Nothing when it has nothing coming.
 *
 * `long` is the board's phrasing, which has room for the whole sentence; a
 * crew card says the same in fewer words.
 */
export function freeLine(agent: string, tasks: Task[], long = false): string | null {
	const checking = tasks.find(
		(t) => t.implementedBy === agent && ['verification', 'review'].includes(t.status)
	);
	if (checking) {
		return long
			? `finished ${checking.id}, its checks are running`
			: `its ${checking.id} is in checks`;
	}
	const next = tasks.find(
		(t) => t.assignedAgent === agent && ['ready', 'assigned', 'waiting'].includes(t.status)
	);
	if (!next) return null;
	const pending = next.dependsOn.filter((id) => tasks.find((t) => t.id === id)?.status !== 'done');
	if (!pending.length) return `takes ${next.id} next`;
	return `takes ${next.id} when ${listIds(pending)} ${pending.length === 1 ? 'lands' : 'land'}`;
}

/** What a planner's line is doing, for the label in front of it on Planning. */
export type PlanningKind = 'reading' | 'thinking' | 'task';

export function planningKind(line: string): PlanningKind {
	if (/^\s*(proposed|task)\b|\bT-\d+\b/i.test(line)) return 'task';
	if (/^\s*(read|open|scan|look)/i.test(line)) return 'reading';
	return 'thinking';
}

/** "no tasks yet", "3 tasks so far", or "9 tasks" once the planner is done. */
export function plannedCount(count: number, running: boolean): string {
	if (count === 0) return running ? FARM_COPY.noTasksYet : 'no tasks';
	const tasks = `${count} ${count === 1 ? 'task' : 'tasks'}`;
	return running ? `${tasks} so far` : tasks;
}
export type TrackState = 'done' | 'current' | 'failed' | 'waiting' | 'soft' | 'yourTurn';
export function readyToLand(detail: TaskDetail | undefined): boolean {
	return (
		!!detail &&
		detail.task.status === 'review' &&
		!!detail.verification?.passed &&
		!detail.verification.unverified &&
		detail.review?.decision === 'approve'
	);
}
export function track(task: Task, detail?: TaskDetail): TrackState[] {
	if (task.status === 'cancelled') return STEPS.map(() => 'soft');
	if (task.status === 'done') return STEPS.map(() => 'done');
	if (readyToLand(detail)) return ['done', 'done', 'done', 'done', 'done', 'yourTurn'];
	let step = {
		draft: 0,
		ready: 0,
		waiting: 0,
		blocked: 0,
		assigned: 1,
		running: 2,
		verification: 3,
		review: 4,
		failed: 2,
		done: 5,
		cancelled: 0
	}[task.status];
	if (task.status === 'failed' || task.status === 'blocked') {
		if (detail?.verification && !detail.verification.passed) step = 3;
		else if (detail?.review && detail.review.decision !== 'approve') step = 4;
		else if (task.worktree) step = 2;
	}
	return STEPS.map((_, i) =>
		i < step
			? 'done'
			: i > step
				? 'soft'
				: task.status === 'failed' || (task.status === 'blocked' && step > 0)
					? 'failed'
					: step === 0
						? 'waiting'
						: 'current'
	);
}
/** Kahn's layers preserve input order. Missing and cyclic dependencies never look runnable. */
export function waves(tasks: Task[]): { tasks: Task[]; unordered: boolean }[] {
	const pending = [...tasks];
	const settled = new Set<string>();
	const result: { tasks: Task[]; unordered: boolean }[] = [];
	while (pending.length) {
		const layer = pending.filter((t) => t.dependsOn.every((id) => settled.has(id)));
		if (!layer.length) {
			result.push({ tasks: [...pending], unordered: true });
			break;
		}
		result.push({ tasks: layer, unordered: false });
		for (const t of layer) {
			settled.add(t.id);
			pending.splice(pending.indexOf(t), 1);
		}
	}
	return result;
}
export function attention(
	task: Task,
	tasks: Task[],
	detail: TaskDetail | undefined,
	runs: AgentRun[]
): boolean {
	if (task.status === 'failed') return true;
	if (task.status === 'blocked')
		return !task.dependsOn.some((id) => tasks.find((t) => t.id === id)?.status !== 'done');
	if (task.status === 'review' && readyToLand(detail))
		return !runs.some((r) => r.task === task.id && r.outcome.state === 'running');
	return (
		task.status === 'review' &&
		!runs.some((r) => r.task === task.id && r.outcome.state === 'running')
	);
}
export function phaseFor(farm: Farm | null, planning: AgentRun | null, crew: number): number {
	if (!farm) return crew ? 1 : 0;
	if (farm.status === 'completed') return 4;
	if (planning || !farm.tasks.some((t) => !['draft', 'cancelled'].includes(t.status))) return 2;
	return 3;
}
const NUMBER_WORDS = [
	'zero',
	'one',
	'two',
	'three',
	'four',
	'five',
	'six',
	'seven',
	'eight',
	'nine'
];
export function numberWord(n: number): string {
	return NUMBER_WORDS[n] ?? String(n);
}
/**
 * Building's first sentence: what is working, what is being checked, and what
 * needs the person. A task that needs the person is counted there only, so a
 * reviewed task waiting for a merge is not also "being checked".
 */
export function nowSentence(tasks: Task[], needs: Task[], paused: boolean): string {
	if (paused) return FARM_COPY.paused;
	const waiting = new Set(needs.map((task) => task.id));
	const working = tasks.filter((t) => t.status === 'running' && !waiting.has(t.id)).length;
	const checking = tasks.filter(
		(t) => ['verification', 'review'].includes(t.status) && !waiting.has(t.id)
	).length;
	const need = needs.length;
	const bits = [
		working ? `${numberWord(working)} ${working === 1 ? 'agent is' : 'agents are'} working` : '',
		checking
			? `${numberWord(checking)} ${checking === 1 ? 'task is' : 'tasks are'} being checked`
			: '',
		need ? `${numberWord(need)} ${need === 1 ? 'needs' : 'need'} you` : ''
	].filter(Boolean);
	const line = bits.length
		? bits.join(', ').replace(/, ([^,]*)$/, ', and $1')
		: tasks.every((t) => t.status === 'done') && tasks.length
			? 'Everything has landed'
			: 'The next task waits for a free agent or an earlier task';
	return line[0].toUpperCase() + line.slice(1) + '.';
}
/** How long ago, as the board says it: "8 s", "6 min", "2 h 14 min". */
export function relativeTime(ms: number, now: number): string {
	return elapsed(Math.max(0, now - ms));
}

/** A span of time in the same words as `relativeTime`: "50 min", not "50m 0s". */
export function elapsed(ms: number): string {
	const seconds = Math.max(0, Math.floor(ms / 1000));
	if (seconds < 60) return `${seconds} s`;
	if (seconds < 3600) return `${Math.floor(seconds / 60)} min`;
	const minutes = Math.floor((seconds % 3600) / 60);
	return `${Math.floor(seconds / 3600)} h${minutes ? ` ${minutes} min` : ''}`;
}
export function clockTime(ms: number): string {
	return ms
		? new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })
		: '—';
}
export function liveRun(task: string, runs: AgentRun[]): AgentRun | undefined {
	return runs.find((r) => r.task === task && r.outcome.state === 'running');
}
export function retryAgent(
	task: Task,
	agents: AgentStatus[],
	runs: AgentRun[]
): AgentStatus | undefined {
	const failed = new Set(
		runs.filter((r) => r.task === task.id && r.outcome.state === 'failed').map((r) => r.agent)
	);
	const available = agents.filter(
		(a) =>
			a.definition.enabled &&
			a.availability.state === 'available' &&
			!runs.some((r) => r.agent === a.definition.id && r.outcome.state === 'running') &&
			!failed.has(a.definition.id)
	);
	return (
		available.find(
			(a) =>
				a.definition.capabilities.includes(task.kind as never) &&
				a.definition.id !== task.implementedBy
		) ??
		available.find(
			(a) => a.definition.capabilities.includes('coding') && a.definition.id !== task.implementedBy
		) ??
		available[0]
	);
}
export function activityLine(event: FarmEvent, agents: AgentStatus[]): string {
	if (event.kind === 'taskAssigned')
		return `${event.task} assigned to ${agentName(event.agent, agents)}`;
	if (event.kind === 'agentStarted')
		return `${agentName(event.agent, agents)} started on ${event.task}`;
	if (event.kind === 'reviewRequested')
		return `${event.task} sent to ${agentName(event.reviewer, agents)} for review`;
	if (event.kind === 'reviewCompleted')
		return `${agentName(event.reviewer, agents)} ${event.approved ? 'approved' : 'asked for changes on'} ${event.task}: ${event.summary}`;
	return eventLine(event);
}
export function eventColour(event: FarmEvent): string {
	if (
		event.kind === 'failed' ||
		(event.kind === 'verificationFinished' && !event.passed) ||
		(event.kind === 'agentStopped' && !event.ok) ||
		(event.kind === 'mergeCompleted' && !event.ok)
	)
		return 'var(--danger)';
	if (event.kind === 'mergeCompleted' || (event.kind === 'reviewCompleted' && event.approved))
		return 'var(--ok)';
	if (event.kind.startsWith('verification') || event.kind.startsWith('review'))
		return 'var(--lane-5)';
	return 'var(--accent)';
}
export function eventTask(event: FarmEvent): string | null {
	return 'task' in event ? event.task : null;
}
export function eventAgent(event: FarmEvent, runs: AgentRun[]): string | null {
	if ('agent' in event) return event.agent;
	if ('reviewer' in event) return event.reviewer;
	if ('run' in event) return runs.find((r) => r.id === event.run)?.agent ?? null;
	return null;
}
export type ActivityFilter = 'All' | 'Needs you' | 'Landed' | 'Failures';
export function filterEvents(
	events: RecordedEvent[],
	filter: ActivityFilter,
	needs: Task[]
): RecordedEvent[] {
	return events
		.filter(
			(e) =>
				e.kind !== 'agentOutput' &&
				(filter === 'All' ||
					(filter === 'Needs you' && needs.some((t) => t.id === eventTask(e))) ||
					(filter === 'Landed' && e.kind === 'mergeCompleted' && e.ok) ||
					(filter === 'Failures' && eventColour(e) === 'var(--danger)'))
		)
		.slice()
		.reverse();
}
