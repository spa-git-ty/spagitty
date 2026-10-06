// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The extension host's wire types (FEAT-096), mirrored by hand from
 * `crates/spagitty-extensions` the way `src/lib/types.ts` mirrors the core.
 * `schemas/extensions/review.v1.schema.json` describes the review model; the
 * fixture in `schemas/extensions/fixtures/review-record.json` is parsed by both
 * test suites so the two sides cannot drift silently.
 */

export type Capability =
	| 'repository.read'
	| 'review.provide'
	| 'tools.execute'
	| 'forge.pullRequest.read'
	| 'forge.pullRequest.comment';

export type ContextKind = 'global' | 'workingCopy' | 'farmTask' | 'pullRequest';

export type Predicate =
	| 'repositoryOpen'
	| 'hasWorkingChanges'
	| 'taskSelected'
	| 'taskHasCommit'
	| 'pullRequestSelected'
	| 'forgeConnected';

export type ReviewTarget = 'workingCopy' | 'farmTask' | 'pullRequest';
export type ReviewScope = 'committed' | 'uncommitted' | 'tracked' | 'includeUntracked';
export type Renderer = 'reviewFindings' | 'reviewStatus' | 'summary';
export type SettingType = 'boolean' | 'enum' | 'text' | 'number' | 'executable';

export interface CommandContribution {
	id: string;
	title: string;
	context?: ContextKind;
	when?: Predicate[];
	menu?: boolean | null;
	palette?: boolean | null;
	reviewProvider?: string | null;
}

export interface ReviewProviderContribution {
	id: string;
	title?: string | null;
	targets: ReviewTarget[];
	scopes?: ReviewScope[];
	configurationFiles?: string[];
	sendsCodeTo?: string | null;
}

export interface PanelContribution {
	id: string;
	title: string;
	renderer: Renderer;
	location?: ContextKind | null;
	provider?: string | null;
}

export interface SettingContribution {
	key: string;
	title?: string | null;
	description?: string | null;
	type: SettingType;
	scope?: 'user' | 'repository';
	default?: unknown;
	values?: string[];
	labels?: Record<string, string>;
	min?: number | null;
	max?: number | null;
	maxLength?: number | null;
	tool?: string | null;
}

export interface Manifest {
	manifestVersion: number;
	id: string;
	name: string;
	version: string;
	publisher: string;
	description?: string | null;
	license?: string | null;
	homepage?: string | null;
	engines: { spagitty: string; extensionApi: string };
	runtime: { kind: string; entrypoints: Record<string, string> };
	activation?: string[];
	capabilities?: { required?: Capability[]; optional?: Capability[] };
	externalTools?: unknown[];
	contributes?: {
		commands?: CommandContribution[];
		reviewProviders?: ReviewProviderContribution[];
		panels?: PanelContribution[];
		settings?: SettingContribution[];
	};
}

export type ExtensionState =
	| 'installed'
	| 'disabled'
	| 'starting'
	| 'active'
	| 'stopping'
	| 'failed'
	| 'incompatible';

export type Provenance = 'bundled' | 'local' | 'development';

export interface CapabilityView {
	capability: Capability;
	required: boolean;
	granted: boolean;
	description: string;
}

export interface SettingView extends SettingContribution {
	value: unknown;
}

export interface Detected {
	found: boolean;
	path?: string;
	version?: string;
	compatible?: boolean;
	reason?: string;
	chosen: boolean;
}

export interface ToolView {
	id: string;
	name: string;
	installUrl: string | null;
	minimumVersion: string | null;
	chosen: string | null;
	detected: Detected | null;
}

export interface ToolRun {
	tool: string;
	profile: string;
	args: string[];
	exitCode: number | null;
	cancelled: boolean;
	timedOut: boolean;
	durationMs: number;
	at: string;
	stderr: string;
}

export interface Diagnostics {
	stderr: string;
	logs: string[];
	toolRuns: ToolRun[];
	error: string | null;
	program: string | null;
}

export interface Compatibility {
	compatible: boolean;
	reasons: string[];
}

export interface ExtensionView {
	id: string;
	name: string;
	version: string;
	publisher: string;
	description: string | null;
	license: string | null;
	homepage: string | null;
	provenance: Provenance;
	official: boolean;
	state: ExtensionState;
	stateReason: string | null;
	compatibility: Compatibility;
	enabled: boolean;
	consented: boolean;
	sendsCodeTo: string[];
	capabilities: CapabilityView[];
	manifest: Manifest;
	settings: SettingView[];
	tools: ToolView[];
	unavailable: { id: string; reason: string }[];
	previousVersion: string | null;
	diagnostics: Diagnostics;
	running: number;
}

export interface Listing {
	extensions: ExtensionView[];
	problems: { source: string; reason: string }[];
	trust: string;
	target: string;
}

export interface PackageFile {
	path: string;
	size: number;
	sha256: string;
	executable: boolean;
}

export interface InstallPreview {
	token: string;
	id: string;
	name: string;
	version: string;
	publisher: string;
	description: string | null;
	license: string | null;
	targets: string[];
	capabilities: CapabilityView[];
	files: PackageFile[];
	digest: string;
	warnings: string[];
	compatibility: Compatibility;
	replaces: string | null;
	addedCapabilities: Capability[];
	trust: string;
}

export interface Installed {
	id: string;
	version: string;
	previous: string | null;
}

export interface Started {
	operation: string;
	reviewId: string | null;
}

export interface Invocation {
	kind: ContextKind;
	workdir?: string | null;
	taskId?: string | null;
	pullRequest?: { number: number; headSha?: string | null } | null;
}

export interface ReviewRequest {
	target: ReviewTarget;
	scope: ReviewScope;
	base?: string | null;
	taskId?: string | null;
	pullRequestNumber?: number | null;
}

export type FileStatus = 'added' | 'modified' | 'deleted' | 'renamed' | 'untracked';

export interface ScopeFile {
	path: string;
	status: FileStatus;
	origin: 'committed' | 'staged' | 'unstaged' | 'untracked';
}

export interface ScopePreview {
	baseRef: string | null;
	baseCommit: string;
	headCommit: string;
	scope: ReviewScope;
	files: ScopeFile[];
	excluded: { path: string; reason: string }[];
	truncated: boolean;
}

// ── The review model ────────────────────────────────────────────────────

export type RunStatus =
	| 'queued'
	| 'running'
	| 'completed'
	| 'incomplete'
	| 'skipped'
	| 'failed'
	| 'cancelled'
	| 'stale'
	| 'actionRequired';

export type Gate = 'notEvaluated' | 'pass' | 'changesRequested' | 'blocked';
export type Severity = 'critical' | 'high' | 'medium' | 'low' | 'info' | 'unknown';
export type Disposition = 'open' | 'acknowledged' | 'dismissed' | 'sentToAgent';

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

export interface ReviewFinding {
	id: string;
	reviewId: string;
	providerId: string;
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
	disposition: Disposition;
}

export interface ReviewResult {
	reviewId: string;
	providerId: string;
	providerVersion: string;
	snapshotId: string;
	status: RunStatus;
	summary: string;
	findings: ReviewFinding[];
	completeness: 'complete' | 'partial' | 'unknown';
	startedAt: string;
	finishedAt?: string;
	actionRequired?: { kind: string; message: string; detail?: unknown };
}

export interface GateDecision {
	gate: Gate;
	reason: string;
	blocking: string[];
}

export interface ReviewRecord {
	schemaVersion: number;
	extension: string;
	extensionVersion: string;
	provider: string;
	requester: 'person' | 'farm';
	snapshot: ReviewSnapshot;
	result: ReviewResult;
	gate?: GateDecision;
	createdMs: number;
}

// ── Events ──────────────────────────────────────────────────────────────

export const EXTENSION_EVENT = 'extension-event';
export const CONFIRM_EVENT = 'extension-confirm';

export type HostEvent =
	| { kind: 'changed'; extension: string }
	| { kind: 'operationStarted'; operation: string; extension: string; reviewId: string | null; title: string }
	| {
			kind: 'operationProgress';
			operation: string;
			extension: string;
			message: string | null;
			elapsedMs: number;
			findings: number;
	  }
	| {
			kind: 'operationFinished';
			operation: string;
			extension: string;
			reviewId: string | null;
			status: string;
			message: string;
	  }
	| { kind: 'notice'; extension: string; level: string; message: string };

export interface Confirmation {
	id: string;
	extension: string;
	kind: 'pullRequestComment';
	title: string;
	target: string;
	body: string;
}

export interface Failure {
	kind: string;
	message: string;
}

// ── Panels ──────────────────────────────────────────────────────────────

export interface SummaryData {
	title?: string;
	rows?: { label: string; value: unknown }[];
	text?: string;
}

export type ObservedState = 'notObserved' | 'requested' | 'running' | 'completed' | 'stale' | 'unavailable';

export interface ReviewStatusItem {
	kind: 'comment' | 'finding' | 'check';
	author?: string;
	authorType?: 'bot' | 'user';
	title?: string;
	body?: string;
	path?: string;
	line?: number;
	url?: string;
	state?: string;
}

export interface ReviewStatusData {
	state: ObservedState;
	headline?: string;
	summary?: string;
	revision?: string;
	items?: ReviewStatusItem[];
	links?: { title: string; url: string }[];
	complete?: boolean;
}
