// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
vi.mock('$lib/api');
import * as api from '$lib/api';
import { timing } from '$lib/timing.svelte';
import TimingsPanel from './TimingsPanel.svelte';

let view: Mounted;
const hold = (seq: number, command: string, waitUs: number, heldUs: number) => ({
	seq,
	atMs: 0,
	command,
	waitUs,
	heldUs
});

beforeEach(() => {
	timing.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.commandTimings).mockResolvedValue([
		hold(1, 'blame', 0, 1_850_000),
		hold(2, 'commit_detail', 1_790_000, 3_000),
		hold(3, 'commit_detail', 200, 5_000)
	]);
});

afterEach(() => view?.destroy());

it('shows what held the repository and what waited for it, the worst first', async () => {
	timing.trip('commit_detail', timing.now() - 1800, true);
	view = render(TimingsPanel, {});
	await vi.waitFor(() => expect(view.text()).toContain('blame'));

	expect(api.commandTimings).toHaveBeenCalledWith(0);
	const tables = view.all('table');
	expect(tables[0].textContent).toMatch(/blame.*1,850 ms/);
	// Only waits of a millisecond or more are counted as waiting.
	expect(tables[1].textContent).toContain('commit_detail');
	expect(tables[1].textContent).toContain('1,790 ms');
	expect(tables[1].querySelectorAll('tbody tr')).toHaveLength(1);
	expect(tables[2].textContent).toContain('commit_detail');
});

it('starts again on Clear', async () => {
	view = render(TimingsPanel, {});
	await vi.waitFor(() => expect(view.text()).toContain('blame'));
	click(view.all('button').find((button) => button.textContent?.trim() === 'Clear')!);
	expect(view.text()).toContain('Nothing has held the repository yet.');
	expect(view.text()).toContain('No calls yet.');
});
