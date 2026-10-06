// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { openRepository } from '../../testing/git-fixtures';
import { branchRow, cleanForecast, conflicts, forecast } from '../../testing/merger-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from '$lib/api';
import { resolving } from '$lib/merger/resolve.svelte';
import { merger } from '$lib/merger/store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import type { MergerReplay } from '$lib/types';
import Page from './+page.svelte';

let view: Mounted;
const text = (element: Element | undefined) => element?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
const button = (name: string) => view.all('button').find((b) => text(b) === name) as HTMLButtonElement;
const cards = () => view.all('article.conflict');

const metrics = conflicts().files[1];
const stopped: MergerReplay = {
	state: 'stopped',
	step: 2,
	total: 5,
	commit: { id: '51d0a7e'.padEnd(40, '0'), short: '51d0a7e', summary: 'feat(chrome): a dragged tab shows where it lands', time: 0 },
	files: [metrics]
};
const done: MergerReplay = { state: 'done', tip: 'd'.repeat(40), short: 'ddddddd', written: 5 };

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	resolving.reset();
	control.reset();
	localStorage.clear();
	localStorage.setItem(
		'spagitty.merger.pick',
		JSON.stringify({ '/repos/fixture': { a: 'main', b: 'feat/tab-drag', into: 'a', strategy: 'rebase' } })
	);
	openRepository();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.branches).mockResolvedValue([branchRow('main', { current: true }), branchRow('feat/tab-drag')]);
	vi.mocked(api.tags).mockResolvedValue([]);
	vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
	vi.mocked(api.mergerRebaseOpen).mockResolvedValue(stopped);
	vi.mocked(api.mergerRebaseContinue).mockResolvedValue(done);
	vi.mocked(api.mergerRebaseSkip).mockResolvedValue(done);
	vi.mocked(api.mergerRebaseAbort).mockResolvedValue(undefined);
	vi.mocked(api.mergerRebaseFinish).mockResolvedValue({ target: 'main', commit: 'd'.repeat(40), short: 'ddddddd', written: 5 });
});

afterEach(() => {
	view?.destroy();
	dialog.dismiss();
	merger.reset();
	resolving.reset();
	control.reset();
});

async function stoppedAt() {
	view = render(Page, {});
	await vi.waitFor(() => expect(button('Resolve 4 conflicts')).toBeDefined());
	click(button('Resolve 4 conflicts'));
	await vi.waitFor(() => expect(cards()).toHaveLength(1));
}

it('stops on a commit and says which, resolving it in the same columns', async () => {
	await stoppedAt();

	expect(api.mergerRebaseOpen).toHaveBeenCalledWith(expect.objectContaining({ strategy: 'rebase', target: 'a' }));
	expect(view.text()).toContain('Rebasing feat/tab-drag onto main · commit 2 of 5');
	expect(view.text()).toContain('51d0a7e feat(chrome): a dragged tab shows where it lands');
	expect(button('Continue · 1 left').disabled).toBe(true);
	expect(button('Skip this commit')).toBeDefined();
});

it('continues once the stop is resolved, then finishes from the dialog', async () => {
	await stoppedAt();

	click(cards()[0].querySelector('button[aria-label="Take main"]')!);
	await vi.waitFor(() => expect(button('Continue').disabled).toBe(false));
	click(button('Continue'));

	await vi.waitFor(() => expect(view.text()).toContain('Ready to land on main'));
	const [, sent] = vi.mocked(api.mergerRebaseContinue).mock.calls[0];
	expect(sent).toEqual([{ path: 'src/lib/metrics.ts', text: 'export const metrics = {\n\trailWidth: 62,\n\ttabHeight: 30,\n\ttabGap: 4,\n\tpaneRadius: 18,\n};\n' }]);
	expect(view.text()).toContain('Each replayed commit keeps its own message.');

	click(button('Finish the rebase'));
	await vi.waitFor(() => expect(view.text()).toContain('main now includes feat/tab-drag'));
	expect(api.mergerRebaseFinish).toHaveBeenCalled();
	expect(api.mergerLand).not.toHaveBeenCalled();
	expect(view.text()).toContain('5 commits replayed with new hashes');
});

it('skips the commit it stopped on', async () => {
	await stoppedAt();
	click(button('Skip this commit'));
	await vi.waitFor(() => expect(api.mergerRebaseSkip).toHaveBeenCalled());
	await vi.waitFor(() => expect(button('Finish the rebase')).toBeDefined());
});

it('aborts, asking first, back to the plan', async () => {
	await stoppedAt();
	click(button('Abort'));
	await vi.waitFor(() => expect(dialog.question?.title).toBe('Abort this rebase'));
	dialog.accept();
	await vi.waitFor(() => expect(api.mergerRebaseAbort).toHaveBeenCalled());
	await vi.waitFor(() => expect(button('Resolve 4 conflicts')).toBeDefined());
});

it('with nothing in the way, Merge now replays and goes straight to the dialog', async () => {
	vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast());
	vi.mocked(api.mergerRebaseOpen).mockResolvedValue(done);
	view = render(Page, {});
	await vi.waitFor(() => expect(button('Merge now')).toBeDefined());

	click(button('Merge now'));

	await vi.waitFor(() => expect(button('Finish the rebase')).toBeDefined());
	expect(cards()).toHaveLength(0);
});
