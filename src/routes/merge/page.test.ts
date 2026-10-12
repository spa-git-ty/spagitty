// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, render, type Mounted } from '../../testing/mount';
import { openRepository } from '../../testing/git-fixtures';
import { branchRow, cleanForecast, forecast } from '../../testing/merger-fixtures';
import { calls, control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from '$lib/api';
import { activity } from '$lib/activity/store.svelte';
import { merger } from '$lib/merger/store.svelte';
import { network } from '$lib/network/store.svelte';
import Page from './+page.svelte';

let view: Mounted;
const text = (element: Element | undefined) => element?.textContent?.replace(/\s+/g, ' ').trim() ?? '';
const button = (name: string) => view.all('button').find((b) => text(b) === name) as HTMLButtonElement;
const result = () => text(view.all('section[aria-label="Result"]')[0]);

beforeEach(() => {
	vi.clearAllMocks();
	merger.reset();
	activity.reset();
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
	expect(view.text()).toContain('Merged into');
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
	// The backend names each side as it was asked for.
	vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast({ b: { ...forecast().b, name: 'old' } }));

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
	vi.mocked(api.mergerForecast).mockResolvedValue(forecast({ b: { ...forecast().b, kind: 'remote' } }));
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('main gets'));

	click(view.all('button.segment')[1]);
	await vi.waitFor(() =>
		expect(result()).toContain('feat/tab-drag is a remote branch; merge into a new branch instead')
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
		expect(text(dialog)).toContain('Ready to merge into main');
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

	it('a rebase replays in its own worktree first, and keeps each message', async () => {
		vi.mocked(api.mergerRebaseOpen).mockResolvedValue({ state: 'done', tip: 'd'.repeat(40), short: 'ddddddd', written: 5 });
		vi.mocked(api.mergerRebaseFinish).mockResolvedValue({ target: 'main', commit: 'd'.repeat(40), short: 'ddddddd', written: 5 });
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('No conflicts'));
		click(view.all('button.strategy').find((b) => text(b).startsWith('Rebase'))!);
		click(button('Merge now'));
		await vi.waitFor(() => expect(api.mergerRebaseOpen).toHaveBeenCalled());
		await vi.waitFor(() => expect(view.text()).toContain('Each replayed commit keeps its own message.'));
		expect(view.all('#merger-message')).toHaveLength(0);
		click(button('Finish the rebase'));
		await vi.waitFor(() => expect(api.mergerRebaseFinish).toHaveBeenCalled());
		expect(vi.mocked(api.mergerRebaseFinish).mock.calls[0][0]).toMatchObject({ strategy: 'rebase', message: undefined });
		expect(api.mergerLand).not.toHaveBeenCalled();
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

/**
 * FEAT-123: the branch that received the merge is checked out, and the one
 * thing left to do is offered.
 */
describe('once it has landed (FEAT-123)', () => {
	const LANDED = { target: 'feat/tab-drag', commit: '4f2c9d1'.padEnd(40, '0'), short: '4f2c9d1', written: 1 };

	beforeEach(() => {
		vi.mocked(api.mergerForecast).mockResolvedValue(cleanForecast());
		vi.mocked(api.mergerLand).mockResolvedValue(LANDED);
		vi.mocked(api.checkout).mockResolvedValue(undefined);
		vi.mocked(api.remotes).mockResolvedValue([
			{ name: 'origin', url: 'git@example.com:a/b.git', pushUrl: null, host: 'generic', refs: 4 }
		]);
	});

	/** Merge `main` into `feat/tab-drag`, which is not the branch checked out. */
	async function landIntoTheOther() {
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('No conflicts'));
		click(button('B Into feat/tab-drag'));
		click(button('Merge now'));
		await vi.waitFor(() => expect(button('Create merge commit')).toBeDefined());
		click(button('Create merge commit'));
		await vi.waitFor(() => expect(view.text()).toContain('feat/tab-drag now includes main'));
	}

	it('checks out the branch it merged into, says so, and offers to push it', async () => {
		await landIntoTheOther();

		await vi.waitFor(() => expect(view.text()).toContain('feat/tab-drag is checked out.'));
		expect(api.checkout).toHaveBeenCalledWith('feat/tab-drag');
		await vi.waitFor(() => expect(button('Push now')).toBeDefined());
		// Where it goes is on the button, not in another sentence.
		expect(button('Push now').title).toBe('Push feat/tab-drag to origin/feat/tab-drag');
		expect(button('Not now')).toBeDefined();
		expect(button('Open in Graph')).toBeDefined();
		// The merge is made: nothing here is called Cancel, which would
		// promise to take it back.
		expect(button('Cancel')).toBeUndefined();
	});

	it('pushes when asked, and then has nothing more to ask', async () => {
		const push = vi.spyOn(network, 'push').mockResolvedValue(true);
		const done = vi.spyOn(network, 'whenDone').mockResolvedValue(true);
		await landIntoTheOther();
		await vi.waitFor(() => expect(button('Push now')).toBeDefined());

		click(button('Push now'));

		await vi.waitFor(() => expect(view.text()).toContain('Pushed to origin/feat/tab-drag.'));
		expect(push).toHaveBeenCalledWith('origin', 'feat/tab-drag');
		expect(button('Push now')).toBeUndefined();
		expect(button('Not now')).toBeUndefined();
		expect(button('Merge another')).toBeDefined();
		push.mockRestore();
		done.mockRestore();
	});

	it('Not now leaves the merge made and unpushed, and closes', async () => {
		const push = vi.spyOn(network, 'push');
		await landIntoTheOther();
		await vi.waitFor(() => expect(button('Not now')).toBeDefined());

		click(button('Not now'));

		await vi.waitFor(() => expect(view.all('[role="dialog"]')).toHaveLength(0));
		expect(push).not.toHaveBeenCalled();
		push.mockRestore();
	});

	it("says why in git's words when the checkout is refused, and offers it again", async () => {
		vi.mocked(api.checkout).mockRejectedValueOnce('Your local changes would be overwritten by checkout');
		await landIntoTheOther();

		await vi.waitFor(() =>
			expect(view.text()).toContain(
				'Merged, but feat/tab-drag could not be checked out: Your local changes would be overwritten by checkout'
			)
		);
		// Still offered: the push names the branch, so it does not need the checkout.
		await vi.waitFor(() => expect(button('Push now')).toBeDefined());

		click(button('Check out feat/tab-drag'));
		await vi.waitFor(() => expect(view.text()).toContain('feat/tab-drag is checked out.'));
		expect(button('Check out feat/tab-drag')).toBeUndefined();
	});

	it('checks nothing out, and remarks on nothing, when it was already the branch here', async () => {
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('No conflicts'));
		vi.mocked(api.mergerLand).mockResolvedValue({ ...LANDED, target: 'main' });
		click(button('Merge now'));
		await vi.waitFor(() => expect(button('Create merge commit')).toBeDefined());
		click(button('Create merge commit'));

		await vi.waitFor(() => expect(button('Push now')).toBeDefined());
		expect(api.checkout).not.toHaveBeenCalled();
		expect(view.text()).not.toContain('checked out');
	});

	it('says there is nowhere to push, and offers no push, with no remote', async () => {
		vi.mocked(api.remotes).mockResolvedValue([]);
		await landIntoTheOther();

		await vi.waitFor(() => expect(view.text()).toContain('No remote to push to.'));
		expect(button('Push now')).toBeUndefined();
		expect(button('Open in Graph')).toBeDefined();
		expect(button('Merge another')).toBeDefined();
	});
});

it('opens on the pair the graph’s drag asked for, the dragged branch coming in (TASK-056)', async () => {
	merger.present({ a: 'feat/tab-drag', b: 'main', into: 'a' });
	view = render(Page, {});
	await vi.waitFor(() => expect(api.mergerForecast).toHaveBeenCalledWith('feat/tab-drag', 'main'));
	expect(merger.into).toBe('a');
});

it('does not show the previous pair’s plan under a new branch name (BUG-061)', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('4 conflicts in 3 files'));

	// The next forecast takes its time.
	let answer!: (value: ReturnType<typeof forecast>) => void;
	vi.mocked(api.mergerForecast).mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));
	merger.pickB('old');
	await vi.waitFor(() => expect(result()).toContain('Working out the merge…'));
	expect(result()).not.toContain('feat/tab-drag');
	expect(view.all('button').find((b) => text(b) === 'Resolve 4 conflicts')).toBeUndefined();
	expect(view.all('button').find((b) => text(b) === 'Merge now')).toBeUndefined();

	answer(forecast({ b: { ...forecast().b, name: 'old' }, conflicts: 0, files: [] }));
	await vi.waitFor(() => expect(result()).not.toContain('Working out the merge…'));
	expect(result()).toContain('old');
});

it('changing only the strategy keeps the plan without a loader (BUG-061)', async () => {
	view = render(Page, {});
	await vi.waitFor(() => expect(result()).toContain('4 conflicts in 3 files'));
	merger.setStrategy('squash');
	await vi.waitFor(() => expect(result()).toContain('after the squash'));
	expect(result()).not.toContain('Working out the merge…');
	expect(api.mergerForecast).toHaveBeenCalledTimes(1);
});

describe('a branch whose upstream has moved (FEAT-126)', () => {
	const card = (which: 'A' | 'B') => view.get(`section[aria-label="Branch ${which}"]`);
	const pullButton = (which: 'A' | 'B') =>
		[...card(which).querySelectorAll('button')].find((b) => /^Pull/.test(text(b))) as HTMLButtonElement | undefined;

	function behind(extra: { main?: object; feat?: object } = {}) {
		vi.mocked(api.branches).mockResolvedValue([
			branchRow('main', { current: true, time: 1_700_000_000, upstream: 'origin/main', ahead: 0, behind: 3, ...extra.main }),
			branchRow('feat/tab-drag', { time: 1_700_000_500, ...extra.feat }),
			branchRow('origin/main', { kind: 'remote', fullName: 'refs/remotes/origin/main' })
		]);
	}

	it('says nothing on a card whose branch is level, or tracks nothing', async () => {
		view = render(Page, {});
		await vi.waitFor(() => expect(result()).toContain('4 conflicts in 3 files'));
		expect(view.find('.behind')).toBeNull();
		expect(pullButton('A')).toBeUndefined();
	});

	it('says how far behind a branch is on its own card, and pulls it on asking', async () => {
		behind();
		vi.mocked(api.pullBranch).mockResolvedValue('');
		view = render(Page, {});
		await vi.waitFor(() => expect(text(card('A'))).toContain('3 new commits on origin/main'));
		expect(card('B').querySelector('.behind')).toBeNull();

		const refreshed = calls.refreshed;
		click(pullButton('A')!);
		await vi.waitFor(() => expect(api.pullBranch).toHaveBeenCalledWith('main'));
		// The refs are read again, so the plan is worked out against the branch as it is now.
		await vi.waitFor(() => expect(calls.refreshed).toBe(refreshed + 1));
		expect(merger.pulling).toBeNull();
		expect(activity.running).toBe(false);
	});

	it('pulls the branch that is not checked out, by its own name', async () => {
		behind({ main: { upstream: null, ahead: null, behind: null }, feat: { upstream: 'origin/feat/tab-drag', ahead: 0, behind: 1 } });
		vi.mocked(api.pullBranch).mockResolvedValue('');
		view = render(Page, {});
		await vi.waitFor(() => expect(text(card('B'))).toContain('1 new commit on origin/feat/tab-drag'));
		expect(card('A').querySelector('.behind')).toBeNull();

		click(pullButton('B')!);
		await vi.waitFor(() => expect(api.pullBranch).toHaveBeenCalledWith('feat/tab-drag'));
	});

	it('does not offer to pull a branch that has commits of its own, and says why', async () => {
		behind({ main: { ahead: 2 } });
		view = render(Page, {});
		await vi.waitFor(() => expect(text(card('A'))).toContain('3 new commits on origin/main'));

		const pull = pullButton('A')!;
		expect(pull.disabled).toBe(true);
		expect(pull.title).toContain('main has 2 commits that origin/main does not');
		click(pull);
		expect(api.pullBranch).not.toHaveBeenCalled();
	});

	it('waits while something else is writing, and says what', async () => {
		behind();
		view = render(Page, {});
		await vi.waitFor(() => expect(pullButton('A')).toBeDefined());

		const step = activity.begin('fetch', 'Fetching', ['network'])!;
		await vi.waitFor(() => expect(pullButton('A')!.disabled).toBe(true));
		expect(pullButton('A')!.title).toBe('Fetching is still running');
		expect(await merger.pullNow('main')).toBe(false);
		expect(api.pullBranch).not.toHaveBeenCalled();
		step.end();
		await vi.waitFor(() => expect(pullButton('A')!.disabled).toBe(false));
	});

	it('lets the lanes go and reads the refs again when git refuses', async () => {
		behind();
		vi.mocked(api.pullBranch).mockRejectedValue('git pull failed: Your local changes would be overwritten');
		view = render(Page, {});
		await vi.waitFor(() => expect(pullButton('A')).toBeDefined());

		const refreshed = calls.refreshed;
		expect(await merger.pullNow('main')).toBe(false);
		expect(activity.running).toBe(false);
		expect(merger.pulling).toBeNull();
		expect(calls.refreshed).toBe(refreshed + 1);
	});
});
