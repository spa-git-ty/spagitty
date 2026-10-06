// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { openRepository } from '../../testing/git-fixtures';
import { branchRow, conflicts, forecast } from '../../testing/merger-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from '$lib/api';
import { resolving } from '$lib/merger/resolve.svelte';
import { merger } from '$lib/merger/store.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import Page from './+page.svelte';

let view: Mounted;
const text = (element: Element | undefined) => element?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
const button = (name: string) => view.all('button').find((b) => text(b) === name) as HTMLButtonElement;
const cards = () => view.all('article.conflict');
const labelled = (label: string, card = 0) =>
	cards()[card].querySelector(`button[aria-label="${label}"]`) as HTMLButtonElement;

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	resolving.reset();
	control.reset();
	localStorage.clear();
	openRepository();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.branches).mockResolvedValue([
		branchRow('main', { current: true }),
		branchRow('feat/tab-drag', { time: 1_700_000_500 })
	]);
	vi.mocked(api.tags).mockResolvedValue([]);
	vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
	vi.mocked(api.mergerConflicts).mockResolvedValue(conflicts());
	vi.mocked(api.mergerState).mockResolvedValue(null);
	vi.mocked(api.setMergerState).mockResolvedValue(undefined);
	vi.mocked(api.mergerLand).mockResolvedValue({ target: 'main', commit: 'c'.repeat(40), short: 'ccccccc', written: 1 });
});

afterEach(() => {
	view?.destroy();
	dialog.dismiss();
	merger.reset();
	resolving.reset();
	control.reset();
});

async function opened() {
	view = render(Page, {});
	await vi.waitFor(() => expect(button('Resolve 4 conflicts')).toBeDefined());
	click(button('Resolve 4 conflicts'));
	await vi.waitFor(() => expect(cards()).toHaveLength(2));
}

it('opens on the first file, in three columns, with every way out', async () => {
	await opened();

	expect(api.mergerConflicts).toHaveBeenCalledWith('main', 'feat/tab-drag');
	expect(view.text()).toContain('Merging feat/tab-drag into main · merge commit');
	expect(view.text()).toContain('0 of 4 resolved');
	const heads = view.all('.column-head').map((head) => text(head));
	expect(heads).toEqual(['Amain· lands here', 'Result· lands in main', 'comes in ·feat/tab-dragB']);
	expect(text(cards()[0])).toContain('Choose what lands here');
	expect(text(cards()[0])).toContain('from e41c0b7 · fix(chrome): hidden repositories leave the tab row');
	for (const label of ['Take main', 'Take feat/tab-drag', 'Both, main first', 'Both, feat/tab-drag first', 'Pick lines from each side', 'Edit the result by hand']) {
		expect(labelled(label)).toBeDefined();
	}
	expect(button('Complete merge · 4 left').disabled).toBe(true);
});

it('every result line says where it came from, and later lines renumber', async () => {
	await opened();
	const first = () => cards()[1].querySelector('.result .row .n')?.textContent;
	expect(first()).toBe('4');

	click(labelled('Both, feat/tab-drag first'));
	await vi.waitFor(() => expect(text(cards()[0])).toContain('Both, feat/tab-drag first'));
	const badges = [...cards()[0].querySelectorAll('.result .own')].map((row) => row.querySelector('.badge')?.textContent);
	expect(badges).toEqual(['B', 'B', 'A']);

	// Three lines where there were none: conflict 2's context moves down by three.
	expect(first()).toBe('7');
});

it('picks single lines, and the unticked ones fade', async () => {
	await opened();

	click(labelled('Pick lines from each side', 1));
	await vi.waitFor(() => expect(cards()[1].querySelectorAll('input[type="checkbox"]').length).toBe(5));
	const ticks = [...cards()[1].querySelectorAll('input[type="checkbox"]')] as HTMLInputElement[];
	click(ticks[2]);
	await vi.waitFor(() => expect(cards()[1].querySelectorAll('.own.dropped').length).toBe(1));
	expect(text(cards()[1])).toContain('Picked lines');
});

it('edits by hand, starting from both sides', async () => {
	await opened();

	click(labelled('Edit the result by hand', 1));
	const box = await vi.waitFor(() => {
		const found = cards()[1].querySelector('textarea') as HTMLTextAreaElement | null;
		expect(found).not.toBeNull();
		return found!;
	});
	expect(box.value.split('\n')).toHaveLength(5);
	box.value = '\t\t<Tab {tab} />';
	box.dispatchEvent(new Event('input', { bubbles: true }));
	await vi.waitFor(() => expect(text(cards()[1])).toContain('Edited by hand'));
});

it('shows the base, steps through the pill, and lands every choice', async () => {
	await opened();

	click(button('Base'));
	await vi.waitFor(() => expect(view.text()).toContain('Base · 29c36a1 · how it looked before either branch'));

	click(labelled('Take main'));
	click(labelled('Take feat/tab-drag', 1));
	click(button('Next unresolved'));
	await vi.waitFor(() => expect(view.text()).toContain('metrics.ts'));
	click(view.all('button[aria-label="Take main here"]')[0]);
	click(button('Next unresolved'));
	await vi.waitFor(() => expect(cards()[0]?.id).toContain('CHANGELOG.md'));
	click(labelled('Both, main first'));

	await vi.waitFor(() => expect(button('Complete merge').disabled).toBe(false));
	expect(view.text()).toContain('4 of 4 resolved');
	await vi.waitFor(() => expect(api.setMergerState).toHaveBeenCalled(), { timeout: 2000 });

	click(button('Complete merge'));
	await vi.waitFor(() => expect(view.text()).toContain('Ready to land on main'));
	expect(view.text()).toContain('Conflict 4 · CHANGELOG.md:3');
	expect(view.text()).toContain('Both, main first');
	click(button('Create merge commit'));

	await vi.waitFor(() => expect(api.mergerLand).toHaveBeenCalled());
	const sent = vi.mocked(api.mergerLand).mock.calls[0][0];
	expect(sent.resolutions.map((r) => r.path)).toEqual(['src/lib/chrome/Tabs.svelte', 'src/lib/metrics.ts', 'CHANGELOG.md']);
	expect(sent.resolutions[2].text).toBe(
		'## Unreleased\n\n- Hidden repositories leave the tab row, and tabs can be pinned.\n- Tabs reorder by dragging, and the order survives a restart.\n\n### Fixed\n'
	);
	await vi.waitFor(() => expect(view.text()).toContain('main now includes feat/tab-drag'));
	expect(api.setMergerState).toHaveBeenLastCalledWith(expect.any(String), null);
});

it('keeps choices for when you come back, and applies them only to the same conflict', async () => {
	const fingerprintless = { version: 1, choices: { 'src/lib/metrics.ts': { '0': { fp: 'nope', choice: { mode: 'a' } } } } };
	vi.mocked(api.mergerState).mockResolvedValue(fingerprintless);
	await opened();
	expect(view.text()).toContain('0 of 4 resolved');

	click(labelled('Take main'));
	click(button('Plan'));
	await vi.waitFor(() => expect(button('Resolve 4 conflicts')).toBeDefined());
	click(button('Resolve 4 conflicts'));
	await vi.waitFor(() => expect(view.text()).toContain('1 of 4 resolved'));
});

it('aborts back to the plan, asking first once something was chosen, and writes nothing', async () => {
	await opened();
	click(labelled('Take main'));

	click(button('Abort'));
	await vi.waitFor(() => expect(dialog.question?.title).toBe('Abort this merge'));
	dialog.accept();

	await vi.waitFor(() => expect(button('Resolve 4 conflicts')).toBeDefined());
	expect(api.mergerLand).not.toHaveBeenCalled();
	expect(resolving.counts.resolved).toBe(0);
});
