// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The words the review screens use (FEAT-096). Pure, so they are tested as a
 * table. "Completed" is never "approved": a run finishing says nothing about
 * whether the code is good, and the gate says that separately.
 */

import type { Gate, ReviewFinding, ReviewRecord, RunStatus, Severity } from './types';

export const STATUS_LABELS: Record<RunStatus, string> = {
	queued: 'Queued',
	running: 'Reviewing',
	completed: 'Reviewed',
	incomplete: 'Incomplete — some files were not reviewed',
	skipped: 'Skipped — no changes to review',
	failed: 'Failed',
	cancelled: 'Cancelled',
	stale: 'Out of date — the code changed during the review',
	actionRequired: 'Needs your action'
};

export const GATE_LABELS: Record<Gate, string> = {
	notEvaluated: 'Not evaluated',
	pass: 'Nothing at or above the threshold',
	changesRequested: 'Changes requested',
	blocked: 'Blocked'
};

export const SEVERITY_ORDER: Severity[] = ['critical', 'high', 'medium', 'low', 'info', 'unknown'];

export const SEVERITY_LABELS: Record<Severity, string> = {
	critical: 'Critical',
	high: 'High',
	medium: 'Medium',
	low: 'Low',
	info: 'Info',
	unknown: 'Unknown'
};

/** Where a finding points, or that it points nowhere in particular. Never a guessed line. */
export function location(finding: ReviewFinding): string {
	if (!finding.path) return 'Whole review';
	if (finding.startLine == null) return finding.path;
	if (finding.endLine != null && finding.endLine !== finding.startLine) {
		return `${finding.path}:${finding.startLine}–${finding.endLine}`;
	}
	return `${finding.path}:${finding.startLine}`;
}

/** Findings ordered most serious first, stable within a severity. */
export function ordered(findings: ReviewFinding[]): ReviewFinding[] {
	return findings
		.map((finding, index) => ({ finding, index }))
		.sort(
			(a, b) =>
				SEVERITY_ORDER.indexOf(a.finding.severity) - SEVERITY_ORDER.indexOf(b.finding.severity) ||
				a.index - b.index
		)
		.map((entry) => entry.finding);
}

/** Counts per severity, most serious first, leaving out the empty ones. */
export function tally(findings: ReviewFinding[]): { severity: Severity; count: number }[] {
	return SEVERITY_ORDER.map((severity) => ({
		severity,
		count: findings.filter((f) => f.severity === severity).length
	})).filter((entry) => entry.count > 0);
}

/** One line for a review: what happened, and how many findings. */
export function headline(record: ReviewRecord): string {
	const status = STATUS_LABELS[record.result.status];
	const count = record.result.findings.length;
	if (record.result.status === 'completed') {
		return count === 0 ? 'Reviewed — no findings' : `Reviewed — ${count} finding${count === 1 ? '' : 's'}`;
	}
	if (count > 0) return `${status} · ${count} finding${count === 1 ? '' : 's'} before it stopped`;
	return status;
}

/** "1m 12s", for a running review. Never a percentage: there is nothing to divide by. */
export function elapsed(ms: number): string {
	const seconds = Math.max(0, Math.floor(ms / 1000));
	const minutes = Math.floor(seconds / 60);
	return minutes ? `${minutes}m ${seconds % 60}s` : `${seconds}s`;
}

/** When a review finished, in the reader's own clock. */
export function when(record: ReviewRecord): string {
	const at = record.result.finishedAt ?? record.result.startedAt;
	const date = new Date(at);
	return Number.isNaN(date.getTime()) ? '' : date.toLocaleString();
}
