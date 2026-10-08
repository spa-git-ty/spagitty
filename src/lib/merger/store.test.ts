// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Merger store's forecast against refreshes (BUG-060): a refresh that
 * moves neither branch must not throw away the answer on its way.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { branchRow, forecast } from '../../testing/merger-fixtures';

vi.mock('$lib/api');

import * as api from '$lib/api';
import { merger } from './store.svelte';
import type { MergerForecast } from '$lib/types';

/** A forecast the test answers when it chooses. */
function pending() {
	let answer!: (value: MergerForecast) => void;
	const promise = new Promise<MergerForecast>((resolve) => (answer = resolve));
	return { promise, answer };
}

function rows(featTip = 'f1') {
	return [
		branchRow('main', { current: true, id: 'm1', time: 1_700_000_000 }),
		branchRow('feat/tab-drag', { id: featTip, time: 1_700_000_500 })
	];
}

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	localStorage.clear();
	vi.mocked(api.tags).mockResolvedValue([]);
});

describe('a refresh while the forecast is on its way', () => {
	it('lets the answer land when neither branch moved', async () => {
		vi.mocked(api.branches).mockResolvedValue(rows());
		const first = pending();
		vi.mocked(api.mergerForecast).mockReturnValueOnce(first.promise);

		const opening = merger.prime('/repo', 'main');
		await vi.waitFor(() => expect(api.mergerForecast).toHaveBeenCalledTimes(1));
		// The watcher refreshes the repository three times before git answers.
		await merger.prime('/repo', 'main');
		await merger.prime('/repo', 'main');
		await merger.prime('/repo', 'main');
		expect(api.mergerForecast).toHaveBeenCalledTimes(1);

		first.answer(forecast());
		await opening;
		expect(merger.forecast).not.toBeNull();
		expect(merger.loading).toBe(false);
	});

	it('asks again when a branch moved, and keeps only the new answer', async () => {
		vi.mocked(api.branches).mockResolvedValueOnce(rows('f1')).mockResolvedValue(rows('f2'));
		const first = pending();
		const second = pending();
		vi.mocked(api.mergerForecast).mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);

		const opening = merger.prime('/repo', 'main');
		await vi.waitFor(() => expect(api.mergerForecast).toHaveBeenCalledTimes(1));
		const moved = merger.prime('/repo', 'main');
		await vi.waitFor(() => expect(api.mergerForecast).toHaveBeenCalledTimes(2));

		first.answer(forecast({ conflicts: 9 }));
		second.answer(forecast({ conflicts: 4 }));
		await Promise.all([opening, moved]);
		expect(merger.forecast?.conflicts).toBe(4);
	});

	it('a settled forecast is kept through a refresh that moved nothing', async () => {
		vi.mocked(api.branches).mockResolvedValue(rows());
		vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
		await merger.prime('/repo', 'main');
		await merger.prime('/repo', 'main');
		expect(api.mergerForecast).toHaveBeenCalledTimes(1);
	});

	it('choosing another branch still asks', async () => {
		vi.mocked(api.branches).mockResolvedValue([...rows(), branchRow('old', { id: 'o1' })]);
		vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
		await merger.prime('/repo', 'main');
		merger.pickB('old');
		await vi.waitFor(() => expect(api.mergerForecast).toHaveBeenCalledTimes(2));
		expect(api.mergerForecast).toHaveBeenLastCalledWith('main', 'old');
	});
});
