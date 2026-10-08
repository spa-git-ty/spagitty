// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What a reviewer has done on one pull request, as it is kept (FEAT-087).
 *
 * Which files were ticked as viewed and at which version of each, the
 * comments written but not yet sent, and what the last look at the pull
 * request learnt about it. Stored by the backend as one JSON file per pull
 * request in Spagitty's application data, so it survives a restart and is the
 * same whichever clone the pull request is opened from.
 *
 * The backend keeps whatever object it is given. This module is the one place
 * that knows the shape, and `normalise` is what stands between a file on disk
 * — possibly from an older build, possibly hand-edited — and the screen.
 */

import type { ForgeRepo, LinePlace, PullRequest } from '../types';
import type { ReviewKey } from '../api';

/** A comment written in the review room and not yet sent. */
export interface PendingComment {
	/** Local identity, so two comments on one line can be told apart. */
	id: string;
	path: string;
	/** The last line the comment covers, in the version `side` names. */
	line: number;
	/** `RIGHT` for the new version's numbering, `LEFT` for the old one's. */
	side: 'LEFT' | 'RIGHT';
	/** The first line, when the comment covers a range. */
	startLine: number | null;
	startSide: 'LEFT' | 'RIGHT' | null;
	body: string;
	/** The head the comment was written against: where its line numbers hold. */
	headSha: string;
	/** Unix seconds. */
	createdAt: number;
	/**
	 * Where the last and the first line sit by both versions' counters, which
	 * is how GitLab places a comment (FEAT-093). Null in a record from before
	 * they were kept; GitHub needs only the numbers.
	 */
	place: LinePlace | null;
	startPlace: LinePlace | null;
	/** The file's path before the change, when it was renamed. */
	oldPath: string | null;
	/**
	 * Set when an agent drafted it (2.0): which assignment and proposal, who,
	 * how sure, and where it stands. A proposed one waits for the person and
	 * does not go out with Finish review; accepted, edited or applied, it is
	 * one of the person's pending comments, with an author.
	 */
	agent?: AgentNote | null;
}

/** What a pending comment drafted by an agent carries. */
export interface AgentNote {
	assignment: string;
	proposal: string;
	/** The agent's name, as the comment says it on the host. */
	name: string;
	severity: 'high' | 'medium' | 'low';
	sure: boolean;
	state: 'proposed' | 'accepted' | 'edited' | 'applied';
}

export interface ReviewRecord {
	version: 1;
	/** The head commit when the pull request was last opened. */
	headSha: string;
	/**
	 * Files ticked as viewed: path to the blob the file had when it was
	 * ticked. A file the author changes afterwards has a different blob, and
	 * reads as unviewed again — which is the whole point of keeping the blob
	 * rather than a yes.
	 */
	viewed: Record<string, string>;
	/** Completed agent steps already synced, so a person's untick stays. */
	agentViewedSteps?: string[];
	/** Comments waiting for Finish review. */
	drafts: PendingComment[];
	/** The text for the pull request as a whole, waiting with them. */
	body: string;
	/** How many files the pull request touched when it was last opened. */
	files: number;
	/**
	 * Files carrying a conflict fix, and the merges they came from, as the
	 * last look found them. Empty until the pull request has been opened once.
	 */
	conflictFiles: string[];
	conflictMerges: string[];
}

export function emptyRecord(): ReviewRecord {
	return {
		version: 1,
		headSha: '',
		viewed: {},
		drafts: [],
		body: '',
		files: 0,
		conflictFiles: [],
		conflictMerges: []
	};
}

function strings(value: unknown): string[] {
	return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : [];
}

function side(value: unknown): 'LEFT' | 'RIGHT' | null {
	return value === 'LEFT' || value === 'RIGHT' ? value : null;
}

function pending(value: unknown): PendingComment | null {
	if (typeof value !== 'object' || value === null) return null;
	const raw = value as Record<string, unknown>;
	const line = Number(raw.line);
	if (typeof raw.path !== 'string' || !Number.isInteger(line) || line < 1) return null;
	if (typeof raw.body !== 'string' || raw.body.trim() === '') return null;
	const endSide = side(raw.side) ?? 'RIGHT';
	const startSide = side(raw.startSide) ?? endSide;
	const asked = raw.startLine === null || raw.startLine === undefined ? null : Number(raw.startLine);
	// A range starts before it ends — on one side. Across the two sides the
	// numbers are different counts, and a removed line can carry a larger one.
	const start =
		asked !== null && Number.isInteger(asked) && asked >= 1 && (asked < line || startSide !== endSide)
			? asked
			: null;
	return {
		id: typeof raw.id === 'string' && raw.id ? raw.id : `${raw.path}:${line}:${Math.random()}`,
		path: raw.path,
		line,
		side: endSide,
		startLine: start,
		startSide: start !== null ? startSide : null,
		body: raw.body,
		headSha: typeof raw.headSha === 'string' ? raw.headSha : '',
		createdAt: Number.isFinite(Number(raw.createdAt)) ? Number(raw.createdAt) : 0,
		place: placeOf(raw.place),
		startPlace: start !== null ? placeOf(raw.startPlace) : null,
		oldPath: typeof raw.oldPath === 'string' && raw.oldPath ? raw.oldPath : null,
		// Only where an agent drafted it: a person's comment reads back as it
		// was written, with no field it never had.
		...withAgent(agentNote(raw.agent))
	};
}

function withAgent(note: AgentNote | null): { agent?: AgentNote } {
	return note ? { agent: note } : {};
}

const SEVERITIES = ['high', 'medium', 'low'] as const;
const NOTE_STATES = ['proposed', 'accepted', 'edited', 'applied'] as const;

function agentNote(value: unknown): AgentNote | null {
	if (typeof value !== 'object' || value === null) return null;
	const raw = value as Record<string, unknown>;
	if (typeof raw.assignment !== 'string' || typeof raw.proposal !== 'string' || typeof raw.name !== 'string') {
		return null;
	}
	const severity = SEVERITIES.find((s) => s === raw.severity) ?? 'low';
	const state = NOTE_STATES.find((s) => s === raw.state) ?? 'proposed';
	return { assignment: raw.assignment, proposal: raw.proposal, name: raw.name, severity, sure: raw.sure !== false, state };
}

function placeOf(value: unknown): LinePlace | null {
	if (typeof value !== 'object' || value === null) return null;
	const raw = value as Record<string, unknown>;
	const count = (n: unknown) => (Number.isInteger(n) && (n as number) >= 0 ? (n as number) : null);
	const old = count(raw.old);
	const fresh = count(raw.new);
	if (raw.kind !== 'added' && raw.kind !== 'removed' && raw.kind !== 'context') return null;
	if (old === null || fresh === null) return null;
	return { kind: raw.kind, old, new: fresh };
}

/**
 * Whatever came off the disk, as a record.
 *
 * Every field is checked on its own and a bad one costs only itself: a review
 * whose viewed ticks were mangled still keeps its pending comments, because
 * losing somebody's unsent words over an unrelated field would be the worst
 * failure this could have.
 */
export function normalise(value: unknown): ReviewRecord {
	const record = emptyRecord();
	if (typeof value !== 'object' || value === null) return record;
	const raw = value as Record<string, unknown>;

	if (typeof raw.headSha === 'string') record.headSha = raw.headSha;
	if (Array.isArray(raw.agentViewedSteps)) record.agentViewedSteps = strings(raw.agentViewedSteps);
	if (typeof raw.viewed === 'object' && raw.viewed !== null && !Array.isArray(raw.viewed)) {
		for (const [path, blob] of Object.entries(raw.viewed as Record<string, unknown>)) {
			if (typeof blob === 'string') record.viewed[path] = blob;
		}
	}
	if (Array.isArray(raw.drafts)) {
		record.drafts = raw.drafts.map(pending).filter((item): item is PendingComment => item !== null);
	}
	if (typeof raw.body === 'string') record.body = raw.body;
	if (Number.isInteger(raw.files) && (raw.files as number) >= 0) record.files = raw.files as number;
	record.conflictFiles = strings(raw.conflictFiles);
	record.conflictMerges = strings(raw.conflictMerges);
	return record;
}

/**
 * Where a pull request's record is kept.
 *
 * From the open repository's forge, or — for a row from "All my repos" — from
 * the `owner/name` the row carries, on the same host. A GitLab project in a
 * nested group has more than one slash; everything before the last is the
 * owner. `host` is the host a search ran against, which is the open
 * repository's when it has one.
 */
export function keyFor(
	repo: ForgeRepo | null,
	pr: PullRequest,
	host: string | null = repo?.host ?? null
): ReviewKey | null {
	if (pr.repository) {
		const cut = pr.repository.lastIndexOf('/');
		if (cut <= 0 || !host) return null;
		return {
			host,
			owner: pr.repository.slice(0, cut),
			name: pr.repository.slice(cut + 1),
			number: pr.number
		};
	}
	if (!repo) return null;
	return { host: repo.host, owner: repo.owner, name: repo.name, number: pr.number };
}

/** One string for a key, for maps. */
export function keyString(key: ReviewKey): string {
	return `${key.host}/${key.owner}/${key.name}#${key.number}`;
}

/** How many files are ticked. */
export function viewedCount(record: ReviewRecord): number {
	return Object.keys(record.viewed).length;
}
