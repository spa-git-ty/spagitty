// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * An agent's findings as pending comments (2.0): pure, so each rule has a
 * test. `agent.svelte.ts` keeps records in step with assignments through
 * these; the room sends with them.
 */

import type { ReviewKey } from '../api';
import type { Assignment, Finding, Proposal } from '$lib/agents/types';
import type { AgentNote, PendingComment, ReviewRecord } from './record';
import type { FileChange } from '../types';

/** A pending comment's id for a proposal: the same every time it is synced. */
export function draftId(assignment: string, proposal: string): string {
	return `agent:${assignment}:${proposal}`;
}

export function keyOf(a: Assignment): ReviewKey | null {
	if (a.target.kind !== 'review') return null;
	const { host, owner, name, number } = a.target;
	return { host, owner, name, number };
}

/** A finding, as a pending comment drafted by `a`'s agent. */
export function draftOf(a: Assignment, proposal: Proposal & { body: { kind: 'comment' } & Finding }): PendingComment {
	const finding = proposal.body;
	const side = finding.side === 'old' ? 'LEFT' : 'RIGHT';
	return {
		id: draftId(a.id, proposal.id),
		path: finding.path,
		line: finding.line,
		side,
		startLine: finding.startLine,
		startSide: finding.startLine !== null ? side : null,
		body: finding.body,
		headSha: a.target.kind === 'review' ? a.target.head : '',
		createdAt: a.startedAt,
		place: null,
		startPlace: null,
		oldPath: null,
		agent: {
			assignment: a.id,
			proposal: proposal.id,
			name: a.agent.name,
			severity: finding.severity,
			sure: proposal.sure,
			state: proposal.state === 'applied' ? 'applied' : 'proposed'
		}
	};
}

/**
 * Bring a record up to an assignment: new findings added, applied ones
 * marked applied, dismissed and superseded ones taken out. A finding the
 * person already decided is left as they left it. Returns whether anything
 * changed, so an unchanged record is not written.
 */
export function syncRecord(record: ReviewRecord, a: Assignment): boolean {
	let changed = false;
	for (const proposal of a.proposals) {
		if (proposal.body.kind !== 'comment') continue;
		const id = draftId(a.id, proposal.id);
		const at = record.drafts.findIndex((d) => d.id === id);
		const gone = proposal.state === 'dismissed' || proposal.state === 'superseded';
		if (gone) {
			if (at >= 0 && record.drafts[at].agent?.state === 'proposed') {
				record.drafts.splice(at, 1);
				changed = true;
			}
			continue;
		}
		if (at < 0) {
			// Decided by the person and since sent or dismissed here: not again.
			if (proposal.decidedBy === 'person') continue;
			record.drafts.push(draftOf(a, proposal as Proposal & { body: { kind: 'comment' } & Finding }));
			changed = true;
			continue;
		}
		const note = record.drafts[at].agent;
		if (note && note.state === 'proposed' && proposal.state === 'applied') {
			record.drafts[at] = { ...record.drafts[at], agent: { ...note, state: 'applied' } };
			changed = true;
		}
	}
	return changed;
}

/** Completed file steps still owed a viewed tick. */
export function completedFiles(record: ReviewRecord, a: Assignment) {
	return a.steps.filter((step) => step.kind.kind === 'file' && step.state === 'done'
		&& !record.agentViewedSteps?.includes(`${a.id}:${step.index}`));
}

/** Tick the blobs the agent actually read, without ticking a newer head. */
export function syncViewed(record: ReviewRecord, a: Assignment, files: FileChange[]): boolean {
	if (a.target.kind !== 'review' || (record.headSha && record.headSha !== a.target.head)) return false;
	let changed = false;
	for (const step of completedFiles(record, a)) {
		if (step.kind.kind !== 'file') continue;
		const path = step.kind.path;
		const file = files.find((file) => file.path === path);
		if (!file) continue;
		record.viewed[file.path] = file.newBlob ?? (file.oldBlob ? `gone:${file.oldBlob}` : `head:${a.target.head}`);
		(record.agentViewedSteps ??= []).push(`${a.id}:${step.index}`);
		changed = true;
	}
	if (changed) {
		record.headSha = a.target.head;
		record.files = files.length;
	}
	return changed;
}

/** The footer an agent's comment carries on the host, when the repository says so. */
export function marked(body: string, note: AgentNote | null | undefined, mark: boolean): string {
	if (!note || !mark || note.state === 'edited') return body;
	return `${body}\n\n_Drafted with ${note.name}_`;
}

/** The words for the whole review, saying how many comments an agent drafted. */
export function reviewBody(body: string, drafts: PendingComment[], mark: boolean): string {
	if (!mark) return body;
	const drafted = drafts.filter((d) => d.agent && d.agent.state !== 'edited');
	if (!drafted.length) return body;
	const names = [...new Set(drafted.map((d) => d.agent!.name))].join(' and ');
	const line =
		drafted.length === 1
			? `1 of these comments was drafted with ${names}.`
			: `${drafted.length} of these comments were drafted with ${names}.`;
	return body.trim() ? `${body.trim()}\n\n${line}` : line;
}

/** Comments that go out with Finish review: everything but what still waits. */
export function sendable(drafts: PendingComment[]): PendingComment[] {
	return drafts.filter((d) => d.agent?.state !== 'proposed');
}
