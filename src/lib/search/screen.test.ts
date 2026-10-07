// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it, vi } from 'vitest';
import { render } from '../../testing/mount';

vi.mock('$lib/api', () => ({ searchStart: vi.fn(), commitDetail: vi.fn(), blame: vi.fn(), inTauri: () => false }));

import LogScreen from './LogScreen.svelte';
import { search } from './store.svelte';
import type { SearchRow } from '$lib/types';

const row = (index: number): SearchRow => ({
	index,
	id: `${index}`.repeat(40),
	short: `${index}`.repeat(7),
	summary: `Commit ${index}`,
	authorName: 'Ada Lovelace',
	authorEmail: 'ada@example.com',
	initials: 'AL',
	time: 1_700_000_000,
	refs: []
});

/** TASK-057: Log in the house style — a head, the question, the results, the commit. */
describe('LogScreen', () => {
	it('heads the screen like every designed one, and asks before anything is run', () => {
		search.clearResults();
		const view = render(LogScreen, {});

		expect(view.get('.title').textContent).toBe('Log');
		expect(view.all('.status')).toHaveLength(0);
		expect(view.text()).toContain('Matches appear as history is walked');
		expect(view.text()).toContain('Open a result to read it');
		view.destroy();
	});

	it('counts the results in the status pill and lists each as a commit', () => {
		search.seed({ author: 'ada', rows: [row(1), row(2)] });
		const view = render(LogScreen, {});

		expect(view.get('.status').textContent).toContain('2 results');
		expect(view.all('.row')).toHaveLength(2);
		expect(view.text()).toContain('author:ada');
		view.destroy();
	});

	it('says which filter was narrowest when nothing matched', () => {
		search.seed({ path: 'src/nowhere.rs', rows: [] });
		const view = render(LogScreen, {});

		expect(view.text()).toContain('Nothing matched');
		expect(view.text()).toContain('path:src/nowhere.rs');
		view.destroy();
	});
});
