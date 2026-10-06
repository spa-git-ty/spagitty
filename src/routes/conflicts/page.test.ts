// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { openRepository, sides, state } from '../../testing/git-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/conflicts/actions');
import * as api from '$lib/api';
import * as actions from '$lib/conflicts/actions';
import { conflicts } from '$lib/conflicts/store.svelte';
import Page from './+page.svelte';

let view: Mounted;
const text = (element: Element | undefined) => element?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
const button = (name: string) => view.all('button').find((b) => text(b) === name) as HTMLButtonElement;

beforeEach(() => {
	vi.clearAllMocks();
	control.reset();
	openRepository();
	conflicts.clear();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.conflicts).mockResolvedValue(state());
	vi.mocked(api.conflictSides).mockImplementation(async (path) => sides(path));
	vi.mocked(api.conflictSettle).mockResolvedValue(undefined);
});

afterEach(() => {
	view?.destroy();
	conflicts.clear();
	control.reset();
});

it('resolves in the three columns, and blocks continuation until every file is marked resolved', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(view.all('article.conflict')).toHaveLength(1));

	expect(view.text()).toContain('shared.txt');
	expect(view.text()).toContain('Choose what lands here');
	expect(button('Continue').disabled).toBe(true);
	expect(button('Continue').title).toBe('Resolve every file first');
	expect(button('Mark resolved').disabled).toBe(true);

	click(view.all('button[aria-label="Take main"]')[0]);
	await vi.waitFor(() => expect(button('Mark resolved').disabled).toBe(false));
	expect(view.text()).toContain('Took main');

	vi.mocked(api.conflicts).mockResolvedValue(state({ files: [] }));
	click(button('Mark resolved'));
	await vi.waitFor(() => expect(api.conflictSettle).toHaveBeenCalledWith('shared.txt', 'one\nOURS\nthree\n', null));
	await vi.waitFor(() => expect(button('Continue').disabled).toBe(false));

	click(button('Continue'));
	expect(actions.continueOperation).toHaveBeenCalledWith('merge');
	click(button('Abort merge'));
	expect(actions.abortOperation).toHaveBeenCalledWith('merge');
});

it('reports failures then recovers to a repository with no operation', async () => {
	vi.mocked(api.conflicts).mockRejectedValueOnce(new Error('index unavailable'));
	view = render(Page, {});
	await vi.waitFor(() => expect(view.text()).toContain('index unavailable'));

	vi.mocked(api.conflicts).mockResolvedValue(state({ operation: 'none', files: [] }));
	click(button('Refresh'));
	await vi.waitFor(() => expect(conflicts.loaded).toBe(true));
	expect(button('Continue')).toBeUndefined();
	expect(view.text()).toContain('Nothing is conflicted.');
});
