// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * One meaning per colour on both views (handoff README): ok passed or runs,
 * waits for a person, running, failed, and muted for skipped, queued and
 * whatever the host decides. Host words differ (GitHub `failure`, GitLab
 * `failed`); the screen does not.
 */
export type Tone = 'ok' | 'waits' | 'running' | 'failed' | 'skipped' | 'muted';

export function tone(state: string): Tone {
	switch (state) {
		case 'runs':
		case 'success':
			return 'ok';
		case 'approval':
		case 'manual':
		case 'waiting':
		case 'action_required':
			return 'waits';
		case 'running':
		case 'in_progress':
			return 'running';
		case 'failure':
		case 'failed':
		case 'timed_out':
		case 'startup_failure':
			return 'failed';
		case 'skipped':
			return 'skipped';
		default:
			return 'muted';
	}
}

/** The glyph beside a job's name, so a state never rests on colour alone. */
export function mark(state: string): string {
	switch (state) {
		case 'runs':
		case 'running':
		case 'in_progress':
			return '●';
		case 'success':
			return '✓';
		case 'approval':
		case 'manual':
		case 'waiting':
		case 'action_required':
			return '‖';
		case 'skipped':
			return '–';
		case 'host':
			return '?';
		case 'failure':
		case 'failed':
		case 'timed_out':
		case 'startup_failure':
			return '✕';
		case 'canceled':
		case 'cancelled':
			return '⊘';
		default:
			return '○';
	}
}

/** A run's or job's state in words, for the run head and hover titles. */
export function word(state: string, result: string): string {
	if (state !== 'completed') return state === 'running' ? 'Running' : 'Queued';
	switch (result) {
		case 'success':
			return 'Passed';
		case 'failure':
		case 'failed':
		case 'timed_out':
			return 'Failed';
		case 'canceled':
		case 'cancelled':
			return 'Cancelled';
		case 'skipped':
			return 'Skipped';
		default:
			return result ? result[0].toUpperCase() + result.slice(1).replace(/_/g, ' ') : 'Done';
	}
}

/** `m:ss`, the way the artboards write a job's time. */
export function clock(seconds: number | null | undefined): string {
	if (seconds == null) return '';
	const whole = Math.max(0, Math.round(seconds));
	return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`;
}

/** `4m`, `2h`, `1d`: how long ago, for the watched list. */
export function ago(unixSeconds: number, now = Date.now()): string {
	if (!unixSeconds) return '';
	const seconds = Math.max(0, Math.floor(now / 1000 - unixSeconds));
	if (seconds < 60) return `${seconds}s`;
	if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
	if (seconds < 86_400) return `${Math.floor(seconds / 3600)}h`;
	return `${Math.floor(seconds / 86_400)}d`;
}
