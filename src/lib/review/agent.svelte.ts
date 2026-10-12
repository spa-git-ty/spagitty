// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Review with an agent (2.0): the agent's work, as the room's own material.
 *
 * A finding is a pending comment (FEAT-093) with an author, kept in the pull
 * request's record beside the person's. Proposed, it waits under its lines
 * and does not go out; accepted, edited or applied at *Sign off*, it is one
 * of the person's pending comments and goes with Finish review like any
 * other. Dismissed, it leaves the record and stays in the assignment's only.
 * So taking over is free: the room is the hand-review room with some drafts
 * already in it.
 */

import * as api from '$lib/api';
import { repo } from '$lib/repo.svelte';
import { CHECK_LABELS } from '$lib/requests/store.svelte';
import { notice } from '$lib/ui/notice.svelte';
import type { PullRequest } from '$lib/types';
import type { ReviewKey } from '$lib/api';
import { agents } from '$lib/agents/store.svelte';
import { consentSlug, isLive } from '$lib/agents/levels';
import type { Assigned, Assignment, Proposal, StartRequest, ThreadBrief } from '$lib/agents/types';
import type { AgentNote, PendingComment, ReviewRecord } from './record';
import { completedFiles, draftId, keyOf, marked, reviewBody, sendable, syncRecord, syncViewed } from './agent-drafts';

export { draftId, draftOf, keyOf, marked, reviewBody, sendable, syncRecord } from './agent-drafts';
import { placeDraft } from './drafts';
import { review } from './store.svelte';

async function sync(a: Assignment): Promise<void> {
	const key = keyOf(a);
	if (!key) return;
	const kept = await review.recordFor(key);
	const probe = structuredClone($state.snapshot(kept)) as ReviewRecord;
	let changed = syncRecord(probe, a);
	let files: Awaited<ReturnType<typeof api.reviewFiles>> = [];
	if (a.target.kind === 'review' && repo.info?.path === a.repo && completedFiles(probe, a).length
		&& (!probe.headSha || probe.headSha === a.target.head)) {
		try {
			files = await api.reviewFiles(a.target.base, a.target.head);
			if (repo.info?.path === a.repo) changed = syncViewed(probe, a, files) || changed;
		} catch {
			// Keep drafts. The viewed ticks can be retried on the next sync.
		}
	}
	if (!changed) return;
	await review.saveRecord(key, (record) => {
		syncRecord(record, a);
		if (repo.info?.path === a.repo) syncViewed(record, a, files);
	});
}

/**
 * The agent's drafts placed in their files' diffs, as Finish review places them
 * in the room: a host that anchors a range by position (GitLab) needs both
 * ends. Best effort — a draft that cannot be placed goes by its line, as before.
 */
async function placed(target: { number: number; target: string; head: string }, drafts: PendingComment[]): Promise<PendingComment[]> {
	const unplaced = drafts.filter((d) => d.agent && !d.place);
	if (!unplaced.length) return drafts;
	const out = new Map<string, PendingComment>();
	try {
		const head = await api.reviewCheckout(target.number, target.target, target.head);
		const changes = await api.reviewFiles(head.mergeBase, head.head);
		for (const path of new Set(unplaced.map((d) => d.path))) {
			const oldPath = changes.find((c) => c.path === path)?.oldPath ?? null;
			try {
				const whole = await api.reviewFile(head.mergeBase, head.head, path, oldPath);
				for (const draft of unplaced.filter((d) => d.path === path)) {
					const done = placeDraft(draft, whole.lines, oldPath);
					if (done) out.set(draft.id, done);
				}
			} catch {
				// This file goes by line.
			}
		}
	} catch {
		// The pull request could not be read: every draft goes by line.
	}
	return drafts.map((d) => out.get(d.id) ?? d);
}

/** Assignments whose last act has been taken, so it is never taken twice. */
const acted = new Set<string>();
/** Assignments already told their head moved. */
const told = new Set<string>();

/** The last act at *Unattended*: send the review, as the person, now. Only
 * for the open repository — the host is reached through it. */
async function lastAct(a: Assignment): Promise<void> {
	if (!a.lastAct || a.lastAct.kind !== 'send' || acted.has(a.id) || a.target.kind !== 'review') return;
	acted.add(a.id);
	const key = keyOf(a)!;
	if (repo.info?.path !== a.repo) {
		await agents.control(a.id, {
			kind: 'acted',
			ok: false,
			message: 'The repository was closed before the review could be sent.'
		});
		return;
	}
	const record = await review.recordFor(key);
	const drafts = await placed(
		a.target,
		sendable(record.drafts).filter((d) => d.headSha === (a.target.kind === 'review' ? a.target.head : ''))
	);
	const mark = agents.rules?.markComments ?? true;
	try {
		await api.submitReview(
			a.target.number,
			a.lastAct.verdict,
			reviewBody(a.lastAct.body, drafts, mark),
			drafts.map((d) => ({
				path: d.path,
				line: d.line,
				side: d.side,
				body: marked(d.body, d.agent, mark),
				startLine: d.startLine,
				startSide: d.startSide,
				place: d.place,
				startPlace: d.startPlace,
				oldPath: d.oldPath
			}))
		);
		const sent = new Set(drafts.map((d) => d.id));
		await review.saveRecord(key, (kept) => {
			kept.drafts = kept.drafts.filter((d) => !sent.has(d.id));
		});
		await agents.control(a.id, { kind: 'acted', ok: true });
		notice.ok(`${a.agent.name} sent the review`, `#${a.target.number} · ${a.lastAct.verdict === 'comment' ? 'Comment' : a.lastAct.verdict === 'approve' ? 'Approve' : 'Request changes'}`);
	} catch (cause) {
		await agents.control(a.id, { kind: 'acted', ok: false, message: `The review was not sent: ${String(cause)}` });
	}
}

let unsubscribe: (() => void) | null = null;

/** Keep records in step with the agents' work. Safe to call twice. */
export function follow(): void {
	if (unsubscribe) return;
	unsubscribe = agents.subscribe((a) => {
		if (a.job !== 'review') return;
		void sync(a);
		void lastAct(a);
	});
}

/** Bring every kept review assignment into its record: after a load. */
export async function syncAll(): Promise<void> {
	for (const a of agents.assignments) {
		if (a.job === 'review') await sync(a);
	}
}

/** Tell a live assignment its pull request's head moved, once. */
export function watchHeads(prs: PullRequest[]): void {
	for (const a of agents.assignments) {
		if (a.target.kind !== 'review' || !isLive(a) || told.has(a.id)) continue;
		const target = a.target;
		const pr = prs.find((p) => p.number === target.number);
		if (pr?.headSha && !target.head.startsWith(pr.headSha) && !pr.headSha.startsWith(target.head)) {
			told.add(a.id);
			void agents.control(a.id, { kind: 'moved' });
		}
	}
}

// ── Deciding ─────────────────────────────────────────────────────────────

async function decideDraft(draft: PendingComment, change: (d: PendingComment) => PendingComment | null): Promise<void> {
	const note = draft.agent;
	if (!note) return;
	const a = agents.assignments.find((x) => x.id === note.assignment);
	const key = a ? keyOf(a) : review.room?.key;
	if (!key) return;
	await review.saveRecord(key, (record) => {
		const at = record.drafts.findIndex((d) => d.id === draft.id);
		if (at < 0) return;
		const next = change(record.drafts[at]);
		if (next) record.drafts[at] = next;
		else record.drafts.splice(at, 1);
	});
}

function tellEngine(note: AgentNote, state: 'accepted' | 'edited' | 'dismissed') {
	const a = agents.assignments.find((x) => x.id === note.assignment);
	if (a && isLive(a)) void agents.control(a.id, { kind: 'decide', proposal: note.proposal, state });
}

export async function accept(draft: PendingComment): Promise<void> {
	if (!draft.agent) return;
	const note = draft.agent;
	await decideDraft(draft, (d) => ({ ...d, agent: { ...note, state: 'accepted' } }));
	tellEngine(note, 'accepted');
}

export async function edit(draft: PendingComment, body: string): Promise<void> {
	if (!draft.agent || !body.trim()) return;
	const note = draft.agent;
	await decideDraft(draft, (d) => ({ ...d, body: body.trim(), agent: { ...note, state: 'edited' } }));
	tellEngine(note, 'edited');
}

export async function dismiss(draft: PendingComment): Promise<void> {
	if (!draft.agent) return;
	const note = draft.agent;
	await decideDraft(draft, () => null);
	tellEngine(note, 'dismissed');
}

export function askWhy(draft: PendingComment): void {
	if (!draft.agent) return;
	void agents.control(draft.agent.assignment, { kind: 'askWhy', proposal: draft.agent.proposal });
}

/** The agent's answer to *Ask why*, once it has given one. */
export function whyOf(draft: PendingComment): string | null {
	const note = draft.agent;
	if (!note) return null;
	const a = agents.assignments.find((x) => x.id === note.assignment);
	return a?.proposals.find((p) => p.id === note.proposal)?.why ?? null;
}

/** Accept every finding of a gate. */
export async function acceptAll(a: Assignment, proposals: Proposal[]): Promise<void> {
	const key = keyOf(a);
	if (!key) return;
	const ids = new Set(proposals.map((p) => draftId(a.id, p.id)));
	await review.saveRecord(key, (record) => {
		record.drafts = record.drafts.map((d) =>
			ids.has(d.id) && d.agent ? { ...d, agent: { ...d.agent, state: 'accepted' } } : d
		);
	});
	for (const p of proposals) {
		if (p.body.kind === 'comment') void agents.control(a.id, { kind: 'decide', proposal: p.id, state: 'accepted' });
	}
}

// ── Assigning ────────────────────────────────────────────────────────────

/**
 * Everything the engine needs for a pull request: its head and merge base,
 * fetched as the room fetches them, its description, threads and checks.
 */
export async function assign(
	pr: PullRequest,
	key: ReviewKey,
	chosen: Assigned,
	resume: string | null = null
): Promise<Assignment | null> {
	const path = repo.info?.path;
	if (!path) return null;
	try {
		const head = await api.reviewCheckout(pr.number, pr.targetBranch, pr.headSha);
		const comments = await api.pullRequestComments(pr.number).catch(() => []);
		const threads: ThreadBrief[] = comments
			.filter((c) => c.inReplyTo === null)
			.map((c) => ({ path: c.path || null, line: c.line, author: c.author, body: c.body, resolved: c.resolved }));
		const record = review.recordAt(key);
		const request: StartRequest = {
			repo: path,
			agent: chosen.agent,
			level: chosen.level,
			note: chosen.note,
			target: {
				kind: 'review',
				host: key.host,
				owner: key.owner,
				name: key.name,
				number: pr.number,
				title: pr.title,
				base: head.mergeBase,
				head: head.head,
				target: pr.targetBranch
			},
			work: {
				job: 'review',
				description: pr.body,
				threads,
				checks: pr.checks ? CHECK_LABELS[pr.checks] : '',
				conflictFixes: record?.conflictFiles ?? []
			},
			lands: false,
			resume
		};
		const remote = agents.snapshot?.remote.find((r) => r.id === chosen.agent) ?? null;
		return await agents.start(request, remote?.providerLabel ?? null, remote ? consentSlug(remote) : null);
	} catch (cause) {
		notice.failed('The agent was not assigned', cause);
		return null;
	}
}

/** Carry a stopped review on from its last finished file. */
export async function resume(a: Assignment, pr: PullRequest, key: ReviewKey): Promise<Assignment | null> {
	acted.delete(a.id);
	told.delete(a.id);
	return assign(pr, key, { agent: a.agent.id, level: a.level, note: '', lands: false }, a.id);
}

/** For the tests. */
export function forgetActed(): void {
	acted.clear();
	told.clear();
	unsubscribe?.();
	unsubscribe = null;
}

