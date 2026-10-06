// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The extension protocol's types (API 1.0.0).
 *
 * `schemas/extensions/protocol.v1.md` is the standard; these are its shapes in
 * TypeScript. Nothing here is Spagitty's private host code: a worker written
 * in another language implements the same page.
 */

export const API_VERSION = '1.0.0';
export const MAX_MESSAGE_BYTES = 1024 * 1024;

export const ErrorCode = {
	ParseError: -32700,
	InvalidRequest: -32600,
	MethodNotFound: -32601,
	InvalidParams: -32602,
	Internal: -32603,
	NotGranted: -32001,
	BadHandle: -32002,
	BadOperation: -32003,
	NotActive: -32004,
	ToolRefused: -32005,
	ToolMissing: -32006,
	Limit: -32007,
	Cancelled: -32008,
	Unsupported: -32010,
	Declined: -32011,
	Uncertain: -32012
} as const;

export type Id = number | string;

export interface RpcError {
	code: number;
	message: string;
	data?: unknown;
}

export type Message =
	| { jsonrpc: '2.0'; id: Id; method: string; params?: unknown }
	| { jsonrpc: '2.0'; method: string; params?: unknown }
	| { jsonrpc: '2.0'; id: Id; result: unknown }
	| { jsonrpc: '2.0'; id: Id; error: RpcError };

export type Capability =
	| 'repository.read'
	| 'review.provide'
	| 'tools.execute'
	| 'forge.pullRequest.read'
	| 'forge.pullRequest.comment';

export type ContextKind = 'global' | 'workingCopy' | 'farmTask' | 'pullRequest';

export interface InvocationContext {
	kind: ContextKind;
	/** An opaque handle; pass it back to the host, never parse it. */
	repository?: string;
	taskId?: string;
	pullRequest?: { number: number; headSha?: string | null };
}

export interface InitializeParams {
	apiVersions: string[];
	host: { name: string; version: string };
	extension: { id: string; version: string };
	session: string;
	capabilities: Capability[];
	locale: string;
	platform: string;
	limits: {
		maxMessageBytes: number;
		maxConcurrentOperations: number;
		inactivityMs: number;
		cancelGraceMs: number;
	};
	settings: Record<string, unknown>;
}

export type ReviewScope = 'committed' | 'uncommitted' | 'tracked' | 'includeUntracked';
export type ReviewTarget = 'workingCopy' | 'farmTask' | 'pullRequest';

export interface ReviewSnapshot {
	id: string;
	repositoryId: string;
	target: ReviewTarget;
	taskId?: string;
	pullRequestNumber?: number;
	baseCommit: string;
	headCommit: string;
	baseRef?: string;
	scope: ReviewScope;
	contentDigest: string;
	configurationDigest?: string;
	takenAt: string;
}

export interface ReviewStartParams {
	operationId: string;
	provider: string;
	reviewId: string;
	repository: string;
	snapshot: ReviewSnapshot;
	workdir: string;
}

export interface CommandParams {
	operationId: string;
	command: string;
	context: InvocationContext;
}

export interface PanelParams {
	panel: string;
	context: InvocationContext;
}

export type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info' | 'unknown';

/** A finding as a worker sends it. The host assigns the review id and owns dispositions. */
export interface Finding {
	id: string;
	severity: Severity;
	providerSeverity?: string;
	path?: string;
	startLine?: number;
	endLine?: number;
	side?: 'old' | 'new';
	title: string;
	message: string;
	suggestion?: string;
	sourceUrl?: string;
	group?: string;
}

export type RunStatus =
	| 'completed'
	| 'incomplete'
	| 'skipped'
	| 'failed'
	| 'cancelled'
	| 'actionRequired';

export interface ReviewOutcome {
	status: RunStatus;
	completeness: 'complete' | 'partial' | 'unknown';
	summary?: string;
	providerVersion?: string;
	actionRequired?: { kind: string; message: string; detail?: unknown };
}

export type OperationStatus = 'completed' | 'failed' | 'cancelled';

export interface Detected {
	found: boolean;
	path?: string;
	version?: string;
	compatible?: boolean;
	reason?: string;
	chosen: boolean;
}

export interface ToolRunResult {
	runId: string;
	exitCode: number | null;
	signalled: boolean;
	timedOut: boolean;
	durationMs: number;
	stderr: string;
}

export interface SummaryPanel {
	title?: string;
	rows?: { label: string; value: unknown }[];
	text?: string;
}

export interface ReviewStatusPanel {
	state: 'notObserved' | 'requested' | 'running' | 'completed' | 'stale' | 'unavailable';
	headline?: string;
	summary?: string;
	revision?: string;
	items?: {
		kind: 'comment' | 'finding' | 'check';
		author?: string;
		authorType?: 'bot' | 'user';
		title?: string;
		body?: string;
		path?: string;
		line?: number;
		url?: string;
		state?: string;
	}[];
	links?: { title: string; url: string }[];
	complete?: boolean;
}

/** Whether a parsed value is a well-formed JSON-RPC 2.0 message. */
export function isMessage(value: unknown): value is Message {
	if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
	const v = value as Record<string, unknown>;
	if (v.jsonrpc !== '2.0') return false;
	if ('method' in v) return typeof v.method === 'string' && v.method.length > 0;
	if (!('id' in v)) return false;
	return ('result' in v) !== ('error' in v);
}

/** Public forge evidence. Resolution is unknown unless the platform supplied it. */
export interface ForgeReviewItem {
 id: number; author: { id: number | null; login: string; kind: string }; body: string; url: string;
 commitSha: string | null; path: string | null; line: number | null; originalLine: number | null;
 side: string | null; resolved: boolean | null; state: string | null; conclusion: string | null;
 createdAt: string | null; appId: number | null; appSlug: string | null; title: string | null;
}
export interface ForgeReviewCollection { items: ForgeReviewItem[]; complete: boolean; error: string | null }
export interface PullRequestSnapshot {
 forge: 'gitHub' | 'gitLab' | 'bitbucket'; host: string; number: number; url: string; headSha: string; baseSha: string;
 discussion: ForgeReviewCollection; findings: ForgeReviewCollection; reviews: ForgeReviewCollection; checks: ForgeReviewCollection; revisionCurrent: boolean;
}
export type CommentReceipt = { status: 'posted'; headSha: string; commentId: number; url?: string } | { status: 'uncertain'; headSha: string; message: string };
export interface ReviewReadiness { ready: boolean; reason?: string; providerVersion?: string }
