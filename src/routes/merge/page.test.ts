// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { openRepository } from '../../testing/git-fixtures';
import { branchRow, cleanForecast, forecast } from '../../testing/merger-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from '$lib/api';
import { merger } from '$lib/merger/store.svelte';
import Page from './+page.svelte';

let view: Mounted;
const text = (element: Element | undefined) => element?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
const button = (name: string) => view.all('button').find((b) => text(b) === name) as HTMLButtonElement;
const result = () => text(view.all('section[aria-label="Result"]')[0]);

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	control.reset();
	localStorage.clear();
	openRepository();
	vi.mocked(api.inTauri).mockReturnValue(true);
	vi.mocked(api.branches).mockResolvedValue([
		branchRow('main', { current: true, time: 1_700_000_000 }),
		branchRow('old', { time: 1_600_000_000 }),
		branchRow('feat/tab-drag', { time: 1_700_000_500 }),
		branchRow('origin/main', { kind: 'remote', fullName: 'refs/remotes/origin/main' })
	]);
	vi.mocked(api.tags).mockResolvedValue([]);
	vi.mocked(api.mergerForecast).mockResolvedValue(forecast());
});

afterEach(() => {
	view?.destroy();
	merger.reset();
	control.reset();
});

it('starts from the checked-out branch and the newest other one, and says it is a dry run', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('4 conflicts in 3 files'));

	expect(api.mergerForecast).toHaveBeenCalledWith('main', 'feat/tab-drag');
	expect(view.text()).toContain('Dry run · nothing written yet');
	expect(view.text()).toContain('Lands here');
	expect(view.text()).toContain('Comes in · stays as is');
	expect(result()).toContain(
		'main gets 5 commits from feat/tab-drag, tied together by one merge commit.'
	);
	expect(view.text()).toContain('5 commits come in');
	expect(view.text()).toContain('continues as');
	expect(view.text()).toContain('7 files touched by either branch · 6 change on main');
});

it('turns the direction round without asking git again', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('main gets'));

	click(button('B Into feat/tab-drag'));
	await vi.waitFor(() => expect(result()).toContain('feat/tab-drag gets 4 commits from main'));
	expect(api.mergerForecast).toHaveBeenCalledTimes(1);

	click(button('Swap'));
	await vi.waitFor(() => expect(result()).toContain('main gets 5 commits'));
});

it('names a new branch, and the result follows what is typed', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('main gets'));

	click(button('Into a new branch'));
	const input = (await vi.waitFor(() => {
		const found = view.all('#merger-new-branch')[0] as HTMLInputElement | undefined;
		expect(found).toBeDefined();
		return found!;
	})) as HTMLInputElement;
	expect(input.value).toBe('merge/main-tab-drag');
	expect(view.text()).toContain('Starting point');
	expect(view.text()).toContain('starts from');

	input.value = 'merge/try';
	input.dispatchEvent(new Event('input', { bubbles: true }));
	await vi.waitFor(() => expect(result()).toContain('A new branch starts from main. merge/try gets'));
});

it('offers every strategy, and says why fast-forward is not one', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('main gets'));

	const ff = view.all('button.strategy').find((b) => text(b).startsWith('Fast-forward only')) as HTMLButtonElement;
	expect(ff.disabled).toBe(true);
	expect(ff.title).toBe('Both branches have commits the other lacks');

	click(view.all('button.strategy').find((b) => text(b).startsWith('Squash'))!);
	await vi.waitFor(() => expect(result()).toContain('main gets one new commit holding everything'));
	expect(view.all('svg[aria-label="History of main after the squash"]')).toHaveLength(1);

	click(view.all('button.strategy').find((b) => text(b).startsWith('Rebase'))!);
	await vi.waitFor(() => expect(result()).toContain('may stop on up to 3 commits'));
});

it('asks again when another branch is picked', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('main gets'));
	vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast({ b: forecast().b }));

	click(view.all('button[aria-label^="Branch B"]')[0]);
	const entry = await vi.waitFor(() => {
		const found = view.all('[role="menuitem"], .entry').find((e) => text(e).startsWith('old'));
		expect(found).toBeDefined();
		return found!;
	});
	click(entry);

	await vi.waitFor(() => expect(api.mergerForecast).toHaveBeenLastCalledWith('main', 'old'));
	await vi.waitFor(() => expect(result()).toContain('No conflicts'));
	expect(button('Merge now')).toBeDefined();
});

it('a remote branch cannot receive the merge', async () => {
	vi.mocked(api.mergerForecast).mockResolvedValue(
		forecast({ b: { ...forecast().b, name: 'origin/main', kind: 'remote' } })
	);
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('main gets'));

	click(view.all('button.segment')[1]);
	await vi.waitFor(() =>
		expect(result()).toContain('origin/main is a remote branch; merge into a new branch instead')
	);
});

it('says what went wrong', async () => {
	vi.mocked(api.mergerForecast).mockRejectedValue('main and feat/tab-drag share no history');
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('share no history'));
});

describe('landing a merge with no conflicts (FEAT-101)', () => {
	beforeEach(() => {
		vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast());
		vi.mocked(api.mergerLand).mockResolvedValue({
			target: 'main',
			commit: '4f2c9d1'.padEnd(40, '0'),
			short: '4f2c9d1',
			written: 1
		});
	});

	it('asks for the message, writes only from the dialog, and says what landed', async () => {
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('No conflicts'));

		click(button('Merge now'));
		const dialog = await vi.waitFor(() => {
			const found = view.all('[role="dialog"]')[0];
			expect(found).toBeDefined();
			return found;
		});
		expect(text(dialog)).toContain('Ready to land on main');
		const message = view.all('#merger-message')[0] as HTMLTextAreaElement;
		expect(message.value).toBe("Merge branch 'feat/tab-drag' into main");
		expect(api.mergerLand).not.toHaveBeenCalled();

		message.value = 'Bring the tab drag in';
		message.dispatchEvent(new Event('input', { bubbles: true }));
		click(button('Create merge commit'));

		await vi.waitFor(() => expect(view.text()).toContain('main now includes feat/tab-drag'));
		expect(api.mergerLand).toHaveBeenCalledWith({
			a: 'main',
			b: 'feat/tab-drag',
			aTip: 'a'.repeat(40),
			bTip: 'b'.repeat(40),
			target: 'a',
			newName: undefined,
			strategy: 'merge',
			message: 'Bring the tab drag in',
			resolutions: []
		});
		expect(view.text()).toContain('One merge commit, 4f2c9d1.');

		click(button('Merge another'));
		await vi.waitFor(() => expect(view.all('[role="dialog"]')).toHaveLength(0));
	});

	it('Back writes nothing, and a refusal is shown in the dialog', async () => {
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('No conflicts'));

		click(button('Merge now'));
		await vi.waitFor(() => expect(button('Back')).toBeDefined());
		click(button('Back'));
		await vi.waitFor(() => expect(view.all('[role="dialog"]')).toHaveLength(0));

		vi.mocked(api.mergerLand).mockRejectedValue('main changed since it was read; reload and try again');
		click(button('Merge now'));
		await vi.waitFor(() => expect(button('Create merge commit')).toBeDefined());
		click(button('Create merge commit'));
		await vi.waitFor(() => expect(view.text()).toContain('main changed since it was read'));
	});

	it('a rebase keeps each message, and a squash names itself', async () => {
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('No conflicts'));
		click(view.all('button.strategy').find((b) => text(b).startsWith('Rebase'))!);
		click(button('Merge now'));
		await vi.waitFor(() => expect(view.text()).toContain('Each replayed commit keeps its own message.'));
		expect(view.all('#merger-message')).toHaveLength(0);
		click(button('Finish the rebase'));
		await vi.waitFor(() => expect(api.mergerLand).toHaveBeenCalled());
		expect(vi.mocked(api.mergerLand).mock.calls[0][0]).toMatchObject({ strategy: 'rebase', message: undefined });
	});

	it('into a new branch sends its name', async () => {
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('No conflicts'));
		click(button('Into a new branch'));
		click(button('Merge now'));
		await vi.waitFor(() => expect(button('Create merge commit')).toBeDefined());
		click(button('Create merge commit'));
		await vi.waitFor(() => expect(api.mergerLand).toHaveBeenCalled());
		expect(vi.mocked(api.mergerLand).mock.calls[0][0]).toMatchObject({
			target: 'new',
			newName: 'merge/main-tab-drag',
			message: "Merge branch 'feat/tab-drag' into merge/main-tab-drag"
		});
	});
});
