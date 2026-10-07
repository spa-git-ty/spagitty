// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Farm screen's state (FEAT-073).
 *
 * # Why the events drive this and not a poll
 *
 * A farm changes when an agent says something, which is at the model's pace and
 * not on any schedule. Polling for it would be either too slow to watch or a
 * request every few hundred milliseconds for an answer that has not changed.
 * So the backend emits, this store applies, and the screen is a function of the
 * store.
 *
 * # Why an event still triggers a refresh
 *
 * Applying an event locally keeps the screen live; refetching keeps it *right*.
 * A status change is applied immediately so the chip moves as it happens, and a
 * snapshot is fetched shortly afterwards so nothing drifts if an event was
 * missed while the window was closed. The refresh is debounced, because a farm
 * with four agents produces bursts.
 *
 * Transcript lines are the exception: they are applied and never refetched, and
 * they are capped, because one agent run produces thousands of them and the
 * point of the pane is the last few hundred.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from './api';
import { attention } from './describe';
import type {
	AgentScore,
	AgentStatus,
	Farm,
	FarmEvent,
	FarmSnapshot,
	RecordedEvent,
	Policy,
	AgentProvider,
	AgentRun,
	StaleWorkspace,
	Task,
	TaskDetail,
	TaskStatus
} from './types';

/** The Tauri event every farm event arrives on. */
export const EVENT = 'farm-event';

/**
 * The task identifier a planning run's output is filed under.
 *
 * A planning run has no task — it is what produces the tasks — so the backend
 * files it under this fixed identifier. It is not in `farm.tasks`, which is why
 * a planning run was invisible until something asked for it by name.
 */
export const PLANNING_TASK = 'planning';

/**
 * How many activity lines are kept.
 *
 * Three hundred: more than a screenful of history, few enough that appending to
 * it stays free. The whole log is on disk and is re-read on open.
 */
export const ACTIVITY_LIMIT = 300;

/** How many transcript lines are kept per task, for the same reason. */
export const TRANSCRIPT_LIMIT = 500;

/** How long a burst of events is allowed to settle before a refetch. */
export const REFRESH_DELAY_MS = 250;

let farm = $state<Farm | null>(null);
let agents = $state<AgentStatus[]>([]);
let undetected = $state<AgentProvider[]>([]);
let activity = $state<RecordedEvent[]>([]);
let runs = $state<AgentRun[]>([]);
let policy = $state<Policy>({ sources: [], text: '' });
let stale = $state<StaleWorkspace[]>([]);
let scoreboard = $state<AgentScore[]>([]);
let waiting = $state<Record<string, string>>({});
let transcripts = $state<Record<string, string[]>>({});
let loaded = $state(false);
let loading = $state(false);
let error = $state<string | null>(null);
let details = $state<Record<string, TaskDetail>>({});
let detailErrors = $state<Record<string, string>>({});
let generation = 0;
/** The repository the store holds, so the rail shows its dot only there. */
let openedPath = $state<string | null>(null);
let openEpoch = 0;
let refreshEpoch = 0;
const detailVersions = new Map<string, string>();
const detailRequests = new Map<string, string>();

let unlisten: UnlistenFn | null = null;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;

function apply(snapshot: FarmSnapshot): void {
	if (farm?.id !== snapshot.farm?.id) {
		generation++;
		details = {};
		detailErrors = {};
		detailVersions.clear();
		detailRequests.clear();
	}
	farm = snapshot.farm;
	agents = snapshot.agents;
	undetected = snapshot.undetected;
	runs = snapshot.runs;
	policy = snapshot.policy;
	scoreboard = snapshot.scoreboard;
	waiting = snapshot.waiting;
	activity = snapshot.events.slice(-ACTIVITY_LIMIT);
	loaded = true;
	void loadDetails();
}

/** Read evidence once per task revision, with no polling and no stale-repository writes. */
async function loadDetails(): Promise<void> {
	const epoch = generation;
	await Promise.all(
		(farm?.tasks ?? []).map(async (task) => {
			const version = `${task.updatedMs}:${task.status}:${task.attempts}`;
			if (detailVersions.get(task.id) === version || detailRequests.get(task.id) === version)
				return;
			detailRequests.set(task.id, version);
			try {
				const found = await api.taskDetail(task.id);
				if (!found) return;
				if (generation !== epoch || detailRequests.get(task.id) !== version) return;
				details = { ...details, [task.id]: found };
				detailVersions.set(task.id, version);
				const next = { ...detailErrors };
				delete next[task.id];
				detailErrors = next;
			} catch (cause) {
				if (generation === epoch)
					detailErrors = { ...detailErrors, [task.id]: api.failure(cause).message };
			} finally {
				if (generation === epoch && detailRequests.get(task.id) === version)
					detailRequests.delete(task.id);
			}
		})
	);
}

/**
 * Apply one event without waiting for a refetch.
 *
 * Only the changes a person watches for. Everything else arrives with the next
 * snapshot, which is a quarter of a second away.
 */
function absorb(event: RecordedEvent): void {
	if (event.kind === 'agentOutput') {
		runs = runs.map((run) =>
			run.id === event.run ? { ...run, lastOutputMs: event.atMs || Date.now() } : run
		);
		const existing = transcripts[event.task] ?? [];
		const next = [...existing, event.line];
		transcripts = {
			...transcripts,
			[event.task]: next.length > TRANSCRIPT_LIMIT ? next.slice(-TRANSCRIPT_LIMIT) : next
		};
		return;
	}

	activity = [...activity, event].slice(-ACTIVITY_LIMIT);

	if (event.kind === 'farmStatusChanged' && farm) {
		farm = { ...farm, status: event.status };
		return;
	}
	if (event.kind === 'taskStatusChanged' && farm) {
		farm = {
			...farm,
			tasks: farm.tasks.map((task) =>
				task.id === event.task ? { ...task, status: event.status, note: event.note } : task
			)
		};
	}
}

function scheduleRefresh(): void {
	if (refreshTimer) clearTimeout(refreshTimer);
	refreshTimer = setTimeout(() => {
		refreshTimer = null;
		void refresh();
	}, REFRESH_DELAY_MS);
}

async function refresh(): Promise<void> {
	const epoch = openEpoch;
	const revision = ++refreshEpoch;
	try {
		const snapshot = await api.snapshot();
		if (epoch === openEpoch && revision === refreshEpoch) apply(snapshot);
	} catch (cause) {
		// A refresh that fails must not blank a screen that is showing
		// something true. The error is recorded and the last snapshot stays.
		error = api.failure(cause).message;
	}
}

export const farmStore = {
	/** The repository this store holds a farm for, if any. */
	get path(): string | null {
		return openedPath;
	},

	get details(): Record<string, TaskDetail> {
		return details;
	},
	get detailErrors(): Record<string, string> {
		return detailErrors;
	},
	loadDetails,
	get farm(): Farm | null {
		return farm;
	},
	get agents(): AgentStatus[] {
		return agents;
	},
	get undetected(): AgentProvider[] {
		return undetected;
	},
	get activity(): RecordedEvent[] {
		return activity;
	},
	get runs(): AgentRun[] {
		return runs;
	},
	get policy(): Policy {
		return policy;
	},
	get stale(): StaleWorkspace[] {
		return stale;
	},
	get scoreboard(): AgentScore[] {
		return scoreboard;
	},

	/**
	 * Why a queued task is not running, from the scheduler itself.
	 *
	 * The screen asks the backend rather than guessing, because the answer
	 * depends on leases and agent availability that only the backend holds.
	 */
	waitingFor(task: string): string | null {
		return waiting[task] ?? null;
	},

	/**
	 * The task list as it is read: parents, each followed by what was cut out
	 * of it (FEAT-076).
	 *
	 * Depth rather than a tree, because the list renders as rows and a row
	 * needs one number to indent by. Two levels are all the farm produces — a
	 * subtask cannot itself be broken down today — but the walk is recursive so
	 * that stays a product decision rather than a shape the interface enforces.
	 */
	get outline(): { task: Task; depth: number; done: number; total: number }[] {
		const tasks = farm?.tasks ?? [];
		const byParent = new Map<string | null, Task[]>();
		for (const task of tasks) {
			const key = task.parent ?? null;
			byParent.set(key, [...(byParent.get(key) ?? []), task]);
		}
		// A child whose parent was deleted is not lost: it is shown at the top
		// level, which is what it has become.
		const known = new Set(tasks.map((task) => task.id));
		const orphans = tasks.filter((task) => task.parent !== null && !known.has(task.parent));

		const rows: { task: Task; depth: number; done: number; total: number }[] = [];
		const walk = (parent: string | null, depth: number): void => {
			for (const task of byParent.get(parent) ?? []) {
				const children = byParent.get(task.id) ?? [];
				rows.push({
					task,
					depth,
					done: children.filter((child) => child.status === 'done').length,
					total: children.length
				});
				walk(task.id, depth + 1);
			}
		};
		walk(null, 0);
		for (const orphan of orphans) rows.push({ task: orphan, depth: 0, done: 0, total: 0 });
		return rows;
	},

	/** Tasks a planner proposed that nobody has accepted or discarded yet. */
	get drafts(): Task[] {
		return (farm?.tasks ?? []).filter((task) => task.status === 'draft');
	},
	get loaded(): boolean {
		return loaded;
	},
	get loading(): boolean {
		return loading;
	},
	get error(): string | null {
		return error;
	},

	get tasks(): Task[] {
		return farm?.tasks ?? [];
	},

	/** Tasks by identifier, for dependency lookups. */
	get byId(): Map<string, Task> {
		return new Map((farm?.tasks ?? []).map((task) => [task.id, task]));
	},

	/** Agents that could be given a task right now. */
	get usable(): AgentStatus[] {
		return agents.filter(
			(agent) => agent.definition.enabled && agent.availability.state === 'available'
		);
	},

	/** How many tasks have finished, and how many there are. */
	get progress(): { done: number; total: number } {
		const tasks = farm?.tasks ?? [];
		return { done: tasks.filter((task) => task.status === 'done').length, total: tasks.length };
	},

	/** Tasks waiting for a person: reviewed and not merged, or blocked. */
	get needsYou(): Task[] {
		return (farm?.tasks ?? []).filter((task) =>
			attention(task, farm?.tasks ?? [], details[task.id], runs)
		);
	},

	/** The lines one task's agent has produced this session. */
	transcript(task: string): string[] {
		return transcripts[task] ?? [];
	},

	/** What the planner has said so far, this session. */
	get planning(): string[] {
		return transcripts[PLANNING_TASK] ?? [];
	},

	/**
	 * The planning run in flight, if there is one.
	 *
	 * Read from `runs` rather than remembered when the status changed, so it
	 * survives the screen being left and come back to, and so the elapsed time
	 * is the run's own rather than the screen's.
	 */
	get planningRun(): AgentRun | null {
		return (
			runs.find((run) => run.phase === 'planning' && run.outcome.state === 'running') ?? null
		);
	},

	/** Tasks in a given status. */
	inStatus(status: TaskStatus): Task[] {
		return (farm?.tasks ?? []).filter((task) => task.status === status);
	},

	/**
	 * Start listening, then point the farm at a repository.
	 *
	 * **In that order** (BUG-022). `farm_open` does not only answer — it starts
	 * agent detection on a thread and reports the result as an event a few
	 * hundred milliseconds later. Subscribing afterwards left a window in which
	 * that event was emitted with nobody listening, and the answer it carried
	 * is the one that decides whether the farm has any agents at all. The
	 * screen then said "Not installed" about agents sitting on `PATH`, with
	 * Plan it disabled, until something unrelated caused a refresh.
	 *
	 * Subscribing first cannot lose it: the listener is armed before the work
	 * that produces the event is asked for.
	 */
	async open(path: string): Promise<void> {
		if (openedPath === path && (loaded || loading)) return;
		this.reset();
		openedPath = path;
		const epoch = ++openEpoch;
		loading = true;
		error = null;
		try {
			await this.listen();
			const snapshot = await api.open(path);
			if (epoch !== openEpoch) return;
			apply(snapshot);
			// Once, on open, and never as part of a refresh: see `leftovers`.
			await this.leftovers();
		} catch (cause) {
			if (epoch === openEpoch) {
				error = api.failure(cause).message;
				openedPath = null;
			}
		} finally {
			if (epoch === openEpoch) loading = false;
		}
	},

	/**
	 * Open a repository's farm before its screen is visited, for the rail dot
	 * (FEAT-109) — but only one that already has a farm, and never in place
	 * of a farm with a run in flight.
	 *
	 * Opening is not free: it detects agents, writes the agent registry into
	 * the repository, and replaces the backend's one farm service, which stops
	 * a planner running elsewhere. A visit to the Farm screen still opens
	 * whatever repository it is in; this only declines to do so unasked.
	 */
	async prime(path: string): Promise<void> {
		if (openedPath === path) return;
		if (runs.some((run) => run.outcome.state === 'running')) return;
		try {
			if (!(await api.exists(path))) return;
		} catch {
			// A farm that cannot be looked for is one the dot cannot speak for.
			return;
		}
		await this.open(path);
	},

	/**
	 * Look for worktrees left behind by tasks no farm claims.
	 *
	 * Asked for by name rather than carried by every snapshot, because
	 * answering it runs `git worktree list` and a snapshot is taken after
	 * every burst of events. Leftovers do not appear while a farm runs — they
	 * are what is left when one stops — so reading them on open and after a
	 * sweep is reading them exactly as often as they can change.
	 */
	async leftovers(): Promise<void> {
		try {
			stale = await api.stale();
		} catch (cause) {
			// A leftovers scan that fails must not blank a working screen.
			error = api.failure(cause).message;
		}
	},

	/** Subscribe to the backend's events. Safe to call twice. */
	async listen(): Promise<void> {
		if (unlisten) return;
		unlisten = await listen<RecordedEvent>(EVENT, (message) => {
			absorb(message.payload);
			// A transcript line changes nothing a snapshot would report, and a
			// run produces thousands of them. Refreshing on each one both cost
			// a round trip per line and, because the refresh is debounced,
			// pushed the refresh that *did* matter past the end of the run
			// (TASK-030).
			if (message.payload.kind !== 'agentOutput') scheduleRefresh();
		});
	},

	/** Stop listening. Running agents are not affected. */
	async stop(): Promise<void> {
		if (refreshTimer) {
			clearTimeout(refreshTimer);
			refreshTimer = null;
		}
		if (unlisten) {
			unlisten();
			unlisten = null;
		}
	},

	refresh,

	/** Apply an event by hand. Exists for the tests and for the event listener. */
	absorb,

	/** Throw away everything. The farm on disk is untouched. */
	reset(): void {
		openEpoch++;
		openedPath = null;
		generation++;
		details = {};
		detailErrors = {};
		detailVersions.clear();
		detailRequests.clear();
		farm = null;
		agents = [];
		undetected = [];
		activity = [];
		runs = [];
		policy = { sources: [], text: '' };
		stale = [];
		scoreboard = [];
		waiting = {};
		transcripts = {};
		loaded = false;
		loading = false;
		error = null;
	},

	/** Record a failure from an action, for the screen to show. */
	fail(cause: unknown): string {
		const failure = api.failure(cause);
		error = failure.message;
		return failure.message;
	},

	clearError(): void {
		error = null;
	}
};
