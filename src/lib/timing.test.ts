// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(() => Promise.resolve()) }));

import { invoke } from '@tauri-apps/api/core';
import * as api from './api';
import { byName, CAPACITY, summarise, timing } from './timing.svelte';

beforeEach(() => {
	timing.clear();
	// Paint at once: what is measured here is the bookkeeping, not a frame.
	vi.stubGlobal('requestAnimationFrame', (run: FrameRequestCallback) => {
		run(0);
		return 0;
	});
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('round trips', () => {
	it('times every call to the backend, answered or refused, under its command', async () => {
		vi.mocked(invoke).mockResolvedValueOnce(undefined).mockRejectedValueOnce('no repository');
		await api.closeRepo();
		await expect(api.snapshot()).rejects.toBe('no repository');

		const trips = timing.trips();
		expect(trips.map((trip) => [trip.name, trip.ok])).toEqual([
			['close_repo', true],
			['snapshot', false]
		]);
		expect(trips.every((trip) => trip.ms >= 0)).toBe(true);
	});

	it('asks for the backend’s timings from where it left off', async () => {
		await api.commandTimings(41);
		expect(invoke).toHaveBeenCalledWith('command_timings', { after: 41 });
	});

	it('keeps only the latest', () => {
		for (let i = 0; i < CAPACITY + 10; i++) timing.trip('graph_request', timing.now(), true);
		expect(timing.trips()).toHaveLength(CAPACITY);
	});
});

describe('measures', () => {
	it('measures from the start to the frame after the change is painted', () => {
		timing.start('diff', 'Cargo.lock', 12000);
		timing.settle('diff');
		expect(timing.measures()).toEqual([expect.objectContaining({ kind: 'diff', label: 'Cargo.lock', size: 12000 })]);
	});

	it('keeps only the latest start of a kind, and ends nothing it did not start', () => {
		timing.settle('navigate');
		timing.start('navigate', '/graph');
		timing.start('navigate', '/review');
		timing.settle('navigate');
		expect(timing.measures().map((measure) => measure.label)).toEqual(['/review']);
	});
});

describe('summaries', () => {
	it('gives the middle, the slow end and the worst', () => {
		const values = Array.from({ length: 100 }, (_, i) => i + 1);
		expect(summarise('x', values)).toEqual({ name: 'x', count: 100, p50: 51, p95: 96, max: 100 });
		expect(summarise('none', [])).toEqual({ name: 'none', count: 0, p50: 0, p95: 0, max: 0 });
	});

	it('groups by name, the worst first', () => {
		const rows = byName(
			[
				{ name: 'blame', ms: 900 },
				{ name: 'commit_detail', ms: 4 },
				{ name: 'commit_detail', ms: 6 }
			],
			(item) => item.name,
			(item) => item.ms
		);
		expect(rows.map((row) => [row.name, row.count, row.max])).toEqual([
			['blame', 1, 900],
			['commit_detail', 2, 6]
		]);
	});
});
