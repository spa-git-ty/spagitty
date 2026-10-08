// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Agents, and the assignments they work on (2.0).
 *
 * One store for Settings › Agents, Review and Merger. It holds the machine
 * list as the backend reads it, the open repository's rules, and every
 * assignment kept for the open repository — live ones updated by event.
 *
 * # No agent, no trace
 *
 * [`agents.usable`] is the one question Review and Merger ask before drawing
 * anything about agents. With no agent set up for a job it is empty, and the
 * screens are exactly the 1.3 screens: no empty panel, no disabled button, no
 * banner. Settings › Agents is the one place that says what agents are.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from './api';
import { inTauri, notifyDesktop } from '$lib/api';
import { dialog } from '$lib/ui/dialog.svelte';
import { notice } from '$lib/ui/notice.svelte';
import { isLive } from './levels';
import type {
	AgentsSnapshot,
	Assignment,
	Control,
	Failure,
	Job,
	LocalAgent,
	RemoteAgent,
	StartRequest
} from './types';

/** Raw output kept per assignment in the webview. The record keeps it all. */
const LINES = 400;

/** An agent that can be offered for a job here. */
export interface Usable {
	id: string;
	name: string;
	reach: 'local' | 'remote';
	/** The model, for a remote agent. */
	detail: string | null;
	local?: LocalAgent;
	remote?: RemoteAgent;
}

type Listener = (assignment: Assignment, before: Assignment | undefined) => void;

function failure(cause: unknown): Failure {
	if (cause && typeof cause === 'object' && 'message' in cause && 'kind' in cause) {
		return cause as Failure;
	}
	return { kind: 'io', message: cause instanceof Error ? cause.message : String(cause) };
}

let snapshot = $state<AgentsSnapshot | null>(null);
let loadedFor = $state<string | null>(null);
let loading = $state(false);
let error = $state<string | null>(null);
let assignments = $state<Assignment[]>([]);
let lines = $state<Record<string, string[]>>({});
let unlisten: UnlistenFn[] = [];
const listeners = new Set<Listener>();

function absorb(next: Assignment): void {
	const index = assignments.findIndex((a) => a.id === next.id);
	const before = index >= 0 ? assignments[index] : undefined;
	if (loadedFor !== null && next.repo !== loadedFor) return;
	if (index >= 0) assignments[index] = next;
	else assignments = [next, ...assignments];
	for (const listener of listeners) listener(next, before);
	announce(next, before);
}

/** Notifications for *waiting for you*, *finished* and *stopped*, only while
 * the window is not focused, each switchable in Settings › Agents. */
function announce(next: Assignment, before: Assignment | undefined): void {
	if (!before || before.state === next.state) return;
	if (typeof document !== 'undefined' && document.hasFocus()) return;
	const notify = snapshot?.notify;
	if (!notify) return;
	const what =
		next.target.kind === 'review' ? `#${next.target.number} ${next.target.title}` : `${next.target.b} into ${next.target.a}`;
	let title: string | null = null;
	if (next.state === 'waiting' && notify.waiting) title = `${next.agent.name} is waiting for you`;
	if (next.state === 'done' && notify.finished) title = `${next.agent.name} finished`;
	if ((next.state === 'stopped' || next.state === 'failed') && notify.stopped) title = `${next.agent.name} stopped`;
	if (title && inTauri()) void notifyDesktop(title, `${what} · ${next.sentence}`).catch(() => {});
}

function hear(id: string, line: string): void {
	const kept = lines[id] ?? [];
	kept.push(line);
	if (kept.length > LINES) kept.splice(0, kept.length - LINES);
	lines[id] = kept;
}

export const agents = {
	get snapshot() {
		return snapshot;
	},
	get loading() {
		return loading;
	},
	get error() {
		return error;
	},
	get assignments() {
		return assignments;
	},
	get rules() {
		return snapshot?.rules ?? null;
	},
	lines(id: string): string[] {
		return lines[id] ?? [];
	},

	/** Read the machine list and, for `repo`, its rules and assignments. */
	async load(repo: string | null): Promise<void> {
		if (!inTauri()) return;
		loading = true;
		error = null;
		try {
			const [next, kept] = await Promise.all([
				api.snapshot(repo),
				repo ? api.list(repo) : Promise.resolve([] as Assignment[])
			]);
			snapshot = next;
			if (loadedFor !== repo) lines = {};
			loadedFor = repo;
			assignments = kept;
			await this.listen();
		} catch (cause) {
			error = failure(cause).message;
		} finally {
			loading = false;
		}
	},

	/** The repository the store last read, so a screen can ask for it again. */
	get repo() {
		return loadedFor;
	},

	async listen(): Promise<void> {
		if (unlisten.length || !inTauri()) return;
		unlisten = [
			await listen<Assignment>('assignment-event', (event) => absorb(event.payload)),
			await listen<{ id: string; line: string }>('assignment-line', (event) =>
				hear(event.payload.id, event.payload.line)
			)
		];
	},

	/** Changes to assignments, for the screens that turn proposals into the
	 * person's material. Returns the way to stop listening. */
	subscribe(listener: Listener): () => void {
		listeners.add(listener);
		return () => listeners.delete(listener);
	},

	/** Agents that may be offered for `job` here: set up, allowed to do the
	 * job, allowed by the repository. Empty means *no agent, no trace*. */
	usable(job: Job): Usable[] {
		if (!snapshot) return [];
		const allowed = (id: string) => snapshot?.rules?.agents?.includes(id) ?? true;
		const out: Usable[] = [];
		for (const local of snapshot.local) {
			if (local.availability.state !== 'available' || !local.jobs[job] || !allowed(local.id)) continue;
			out.push({ id: local.id, name: local.name, reach: 'local', detail: null, local });
		}
		for (const remote of snapshot.remote) {
			if (!remote.jobs[job] || !allowed(remote.id)) continue;
			out.push({ id: remote.id, name: remote.name, reach: 'remote', detail: remote.model, remote });
		}
		return out;
	},

	/** The default agent for a job, when it is still usable. */
	defaultFor(job: Job): string | null {
		const wanted = job === 'review' ? snapshot?.defaults.review : snapshot?.defaults.merge;
		const usable = this.usable(job);
		return usable.find((u) => u.id === wanted)?.id ?? usable[0]?.id ?? null;
	},

	/** The assignment for a pull request: the live one, else the latest. */
	forReview(owner: string, name: string, number: number): Assignment | null {
		const matching = assignments.filter(
			(a) =>
				a.target.kind === 'review' &&
				a.target.owner === owner &&
				a.target.name === name &&
				a.target.number === number
		);
		return matching.find(isLive) ?? matching[0] ?? null;
	},

	/** The assignment for a merge of `b` into `a`. */
	forMerge(a: string, b: string): Assignment | null {
		const matching = assignments.filter(
			(x) => x.target.kind === 'merge' && x.target.a === a && x.target.b === b
		);
		return matching.find(isLive) ?? matching[0] ?? null;
	},

	/** Is anything here waiting for the person in this job? The rail's dot. */
	waiting(job: Job): boolean {
		return assignments.some((a) => a.job === job && a.state === 'waiting');
	},

	/**
	 * Start an assignment. A remote agent in a repository that has not agreed
	 * asks first — once, here — and the answer is kept in Settings › Agents.
	 */
	async start(request: StartRequest, providerLabel: string | null, slug: string | null): Promise<Assignment | null> {
		try {
			const started = await api.start(request);
			absorb(started);
			return started;
		} catch (cause) {
			const why = failure(cause);
			if (why.kind === 'consent' && slug) {
				const agreed = await dialog.confirm({
					title: `Send code to ${providerLabel ?? 'the provider'}?`,
					body: `${why.message} You can change this in Settings › Agents.`,
					confirmLabel: 'Agree'
				});
				if (!agreed) return null;
				await api.consent(request.repo, slug);
				await this.load(request.repo);
				return this.start(request, providerLabel, null);
			}
			notice.failed('The agent was not assigned', why.message);
			return null;
		}
	},

	async control(id: string, control: Control): Promise<void> {
		try {
			await api.control(id, control);
		} catch (cause) {
			notice.failed('The agent did not take that', failure(cause).message);
		}
	},

	async forget(id: string): Promise<void> {
		const at = assignments.find((a) => a.id === id);
		if (!at) return;
		try {
			await api.forget(at.repo, id);
			assignments = assignments.filter((a) => a.id !== id);
		} catch (cause) {
			notice.failed('Not forgotten', failure(cause).message);
		}
	},

	/** For the tests: an assignment, as the event would bring it. */
	absorb,
	failure,

	/** For the tests: start from nothing. */
	reset(next: AgentsSnapshot | null = null, kept: Assignment[] = []): void {
		snapshot = next;
		assignments = kept;
		lines = {};
		loadedFor = null;
		error = null;
	}
};
