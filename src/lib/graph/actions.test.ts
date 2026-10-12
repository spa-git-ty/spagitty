// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What the graph's menus actually do.
 *
 * The module's own doc comment sets the contract: ask when there is something
 * to lose, run it, re-read. The branches that matter are the *refusals* — a
 * cancelled confirmation must not reach `api`, and a failed operation must not
 * refresh as though it had worked. Both are invisible in the UI until the day
 * they are wrong, and both destroy work when they are.
 *
 * `api`, the stores and the dialog are mocked; what is under test is the
 * decision-making between them, not git.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ResetMode, StashAction } from '$lib/types';

/** The options object every action hands to the dialog, as far as tests care. */
interface Asked {
	title: string;
	body: string;
	confirmLabel: string;
	danger?: boolean;
	value?: string;
}

const confirm = vi.fn<(options: Asked) => Promise<boolean>>();
const prompt = vi.fn<(options: Asked) => Promise<string | null>>();
vi.mock('$lib/ui/dialog.svelte', () => ({
	dialog: {
		confirm: (options: Asked) => confirm(options),
		prompt: (options: Asked) => prompt(options)
	}
}));

const ok = vi.fn();
const failed = vi.fn();
vi.mock('$lib/ui/notice.svelte', () => ({
	notice: {
		ok: (title: string, detail?: string | null) => ok(title, detail),
		failed: (title: string, error: unknown) => failed(title, error)
	}
}));

const reload = vi.fn(() => Promise.resolve());
vi.mock('$lib/graph/store.svelte', () => ({ graph: { reload: () => reload() } }));

/**
 * The network store is a collaborator here, not the thing under test — and it
 * is a module singleton holding a live token, so a real one would still be
 * running the first test's fetch during the second test's push.
 */
const startFetch = vi.fn(() => Promise.resolve(true));
const startPush = vi.fn(() => Promise.resolve(true));
let startError: string | null = null;
vi.mock('$lib/network/store.svelte', () => ({
	network: {
		get error() {
			return startError;
		},
		fetch: () => startFetch(),
		push: () => startPush()
	}
}));

const repoRefresh = vi.fn(() => Promise.resolve());

let working = 0;
vi.mock('$lib/repo.svelte', () => ({
	repo: {
		refresh: () => repoRefresh(),
		get counts() {
			return { working };
		},
		get info() {
			return { head: { branch: 'feature/tabs', short: 'abc1234' } };
		}
	}
}));

let confirmHistoryRewrite = true;
vi.mock('$lib/settings/store.svelte', () => ({
	settings: {
		get settings() {
			return { confirmHistoryRewrite };
		}
	}
}));

/**
 * `vi.hoisted`, because `vi.mock`'s factory is lifted above every `const` in
 * this file and `() => api` would dereference the binding before it exists.
 */
const api = vi.hoisted(() => ({
	pull: vi.fn(() => Promise.resolve('')),
	stashPush: vi.fn(() => Promise.resolve()),
	reset: vi.fn(() => Promise.resolve()),
	revert: vi.fn(() => Promise.resolve()),
	cherryPick: vi.fn(() => Promise.resolve()),
	rebaseOnto: vi.fn(() => Promise.resolve()),
	checkout: vi.fn(() => Promise.resolve()),
	checkoutDetached: vi.fn(() => Promise.resolve()),
	createBranch: vi.fn(() => Promise.resolve()),
	createTag: vi.fn(() => Promise.resolve()),
	renameBranch: vi.fn(() => Promise.resolve()),
	deleteBranch: vi.fn(() => Promise.resolve()),
	deleteTag: vi.fn(() => Promise.resolve()),
	stashAction: vi.fn(() => Promise.resolve()),
	fetch: vi.fn(() => Promise.resolve('')),
	push: vi.fn(() => Promise.resolve(''))
}));
vi.mock('$lib/api', () => api);

import * as actions from '$lib/graph/actions';
import { activity } from '$lib/activity/store.svelte';

/** The last options object handed to `dialog.confirm`. */
function lastConfirmation(): Asked {
	const call = confirm.mock.calls.at(-1);
	if (!call) throw new Error('nothing was confirmed');
	return call[0];
}

beforeEach(() => {
	vi.clearAllMocks();
	working = 0;
	confirmHistoryRewrite = true;
	confirm.mockResolvedValue(true);
	prompt.mockResolvedValue('given-name');
	activity.reset();
});

/** A call the test finishes when it chooses. */
function held() {
	let finish!: () => void;
	const promise = new Promise<void>((resolve) => (finish = resolve));
	return { promise, finish };
}

/**
 * FEAT-121. Every one of these ran with nothing on screen until it ended, and
 * a second press started a second git over the first.
 */
describe('said while it runs (FEAT-121)', () => {
	it('names the work for as long as git takes, and lets go when it ends', async () => {
		const git = held();
		api.checkout.mockReturnValueOnce(git.promise);

		const running = actions.checkoutBranch('main');
		await vi.waitFor(() => expect(activity.label).toBe('Switching to main…'));
		expect(activity.is('checkout')).toBe(true);

		git.finish();
		await running;
		expect(activity.running).toBe(false);
	});

	it('lets go when git refuses', async () => {
		api.checkout.mockRejectedValueOnce('your local changes would be overwritten');

		await actions.checkoutBranch('main');

		expect(activity.running).toBe(false);
		expect(failed).toHaveBeenCalledWith(
			'Could not switch to main',
			'your local changes would be overwritten'
		);
	});

	it('does not start a second write over one still running, and says which', async () => {
		activity.begin('pull', 'Pulling', ['repository', 'network']);

		await actions.checkoutBranch('main');

		expect(api.checkout).not.toHaveBeenCalled();
		expect(failed).toHaveBeenCalledWith('Could not switch to main', 'Pulling is still running');
		expect(ok).not.toHaveBeenCalled();
		expect(repoRefresh).not.toHaveBeenCalled();
	});

	it('holds nothing while a question is still open', async () => {
		// The lane is taken when git starts, not when a dialog does: a
		// confirmation left open must not freeze the toolbar behind it.
		let answer!: (agreed: boolean) => void;
		confirm.mockReturnValueOnce(new Promise<boolean>((resolve) => (answer = resolve)));

		const running = actions.revertCommit('a'.repeat(40), 'aaaaaaa');
		await Promise.resolve();
		expect(activity.running).toBe(false);

		answer(false);
		await running;
		expect(activity.running).toBe(false);
	});

	it('names a stash as a stash, so the toolbar knows whose button it is', async () => {
		working = 3;
		prompt.mockResolvedValueOnce('WIP');
		const git = held();
		api.stashPush.mockReturnValueOnce(git.promise);

		const running = actions.quickStash();
		await vi.waitFor(() => expect(activity.is('stash')).toBe(true));
		expect(activity.label).toBe('Stashing your changes…');

		git.finish();
		await running;
	});

	it.each([
		['fastForwardOnly', 'Pulling…'],
		['rebase', 'Pulling and rebasing…']
	] as const)('names a %s pull and holds both lanes for it', async (mode, label) => {
		const seen: { label: string; repository: string | null; network: string | null }[] = [];
		api.pull.mockImplementationOnce(async () => {
			seen.push({
				label: activity.label,
				repository: activity.blocked('repository'),
				network: activity.blocked('network')
			});
			return '';
		});

		await actions.pull(mode);

		expect(seen).toHaveLength(1);
		expect(seen[0].label).toBe(label);
		expect(seen[0].repository).not.toBeNull();
		expect(seen[0].network).not.toBeNull();
		expect(activity.running).toBe(false);
	});

	it('says each of the three parts of a pull over uncommitted work', async () => {
		working = 2;
		const said: string[] = [];
		api.stashPush.mockImplementationOnce(async () => void said.push(activity.label));
		api.pull.mockImplementationOnce(async () => {
			said.push(activity.label);
			return '';
		});
		api.stashAction.mockImplementationOnce(async () => void said.push(activity.label));

		await actions.pull();

		expect(said).toEqual(['Stashing your changes…', 'Pulling…', 'Putting your changes back…']);
		expect(activity.running).toBe(false);
	});

	it('lets go when the pull fails with the changes in the stash', async () => {
		working = 2;
		api.pull.mockRejectedValueOnce('Not possible to fast-forward');

		await actions.pull();

		expect(activity.running).toBe(false);
	});

	it('lets go when the stash before a pull fails', async () => {
		working = 2;
		api.stashPush.mockRejectedValueOnce('could not write index');

		await actions.pull();

		expect(api.pull).not.toHaveBeenCalled();
		expect(activity.running).toBe(false);
	});

	it('does not pull while a fetch is running, and asks nothing first', async () => {
		working = 2;
		activity.begin('fetch', 'Fetching', ['network']);

		await actions.pull();

		expect(confirm).not.toHaveBeenCalled();
		expect(api.stashPush).not.toHaveBeenCalled();
		expect(api.pull).not.toHaveBeenCalled();
		expect(failed).toHaveBeenCalledWith('Could not pull', 'Fetching is still running');
	});

	it('does not pull over something that started while the question was open', async () => {
		working = 2;
		confirm.mockImplementationOnce(async () => {
			activity.begin('stash', 'Stashing your changes');
			return true;
		});

		await actions.pull();

		expect(api.stashPush).not.toHaveBeenCalled();
		expect(api.pull).not.toHaveBeenCalled();
		expect(failed).toHaveBeenCalledWith('Could not pull', 'Stashing your changes is still running');
	});

	it('does not fetch or push while a pull is running, and says so', async () => {
		activity.begin('pull', 'Pulling', ['repository', 'network']);

		await actions.fetchAll();
		await actions.pushCurrent();

		expect(startFetch).not.toHaveBeenCalled();
		expect(startPush).not.toHaveBeenCalled();
		expect(failed).toHaveBeenCalledWith('Could not fetch', 'Pulling is still running');
		expect(failed).toHaveBeenCalledWith('Could not push', 'Pulling is still running');
	});

	it('fetches and pushes while only the working copy is being written', async () => {
		activity.begin('stash', 'Stashing your changes');

		await actions.fetchAll();

		expect(startFetch).toHaveBeenCalled();
		expect(failed).not.toHaveBeenCalled();
	});
});

describe('perform — the shape every action shares', () => {
	it('reports and re-reads on success', async () => {
		await actions.checkoutBranch('main');

		expect(api.checkout).toHaveBeenCalledWith('main');
		expect(ok).toHaveBeenCalledWith('Switched to main', undefined);
		expect(repoRefresh).toHaveBeenCalledTimes(1);
		expect(reload).toHaveBeenCalledTimes(1);
	});

	/** A refresh after a failure redraws the same state and reads as success. */
	it('reports the failure and does not re-read', async () => {
		const boom = new Error('would be overwritten');
		api.checkout.mockRejectedValueOnce(boom);

		await actions.checkoutBranch('main');

		expect(failed).toHaveBeenCalledWith('Could not switch to main', boom);
		expect(ok).not.toHaveBeenCalled();
		expect(reload).not.toHaveBeenCalled();
	});

	it('does not throw out to a menu entry, which has nowhere to catch', async () => {
		api.checkout.mockRejectedValueOnce(new Error('nope'));
		await expect(actions.checkoutBranch('main')).resolves.toBeUndefined();
	});
});

describe('copyId', () => {
	it('puts the full id on the clipboard and names the short one', async () => {
		const writeText = vi.fn(() => Promise.resolve());
		vi.stubGlobal('navigator', { clipboard: { writeText } });

		await actions.copyId('a1b2c3d4e5f6', 'a1b2c3d');

		expect(writeText).toHaveBeenCalledWith('a1b2c3d4e5f6');
		expect(ok).toHaveBeenCalledWith('Copied a1b2c3d', undefined);
		vi.unstubAllGlobals();
	});

	it('says so when the webview refuses clipboard access', async () => {
		const denied = new Error('denied');
		vi.stubGlobal('navigator', { clipboard: { writeText: vi.fn(() => Promise.reject(denied)) } });

		await actions.copyId('a1b2c3d4e5f6', 'a1b2c3d');

		expect(failed).toHaveBeenCalledWith('Could not copy the commit id', denied);
		vi.unstubAllGlobals();
	});
});

describe('createBranchNamed / createTagAt', () => {
	it('creates and checks out the branch named in the graph (FEAT-104)', async () => {
		expect(await actions.createBranchNamed('feature/FEAT-031-graph', 'a1b2c3d4')).toBe(true);

		expect(prompt).not.toHaveBeenCalled();
		expect(api.createBranch).toHaveBeenCalledWith('feature/FEAT-031-graph', 'a1b2c3d4', true);
		expect(ok).toHaveBeenCalledWith('Created feature/FEAT-031-graph', undefined);
	});

	it('tags the commit it was given a name for', async () => {
		prompt.mockResolvedValueOnce('v2.1.0');

		await actions.createTagAt('a1b2c3d4', 'a1b2c3d');

		expect(api.createTag).toHaveBeenCalledWith('v2.1.0', 'a1b2c3d4');
		expect(ok).toHaveBeenCalledWith('Tagged a1b2c3d as v2.1.0', undefined);
	});

	it('tags nothing when the naming prompt is dismissed', async () => {
		prompt.mockResolvedValueOnce(null);

		await actions.createTagAt('a1b2c3d4', 'a1b2c3d');

		expect(api.createTag).not.toHaveBeenCalled();
	});
});

describe('resetTo', () => {
	/**
	 * The module's rule: `hard` always asks, whatever the setting says, because
	 * uncommitted work destroyed by it is the one thing the reflog cannot return.
	 */
	it('always asks before a hard reset, even with the warning turned off', async () => {
		confirmHistoryRewrite = false;

		await actions.resetTo('a1b2c3d4', 'a1b2c3d', 'hard');

		expect(confirm).toHaveBeenCalledTimes(1);
		expect(lastConfirmation().danger).toBe(true);
		expect(lastConfirmation().body).toContain('cannot be recovered');
		expect(api.reset).toHaveBeenCalledWith('a1b2c3d4', 'hard');
	});

	it.each<[ResetMode, string]>([
		['soft', 'staged and ready to commit again'],
		['mixed', 'in your files, unstaged']
	])('asks before a %s reset while the warning is on', async (mode, wording) => {
		await actions.resetTo('a1b2c3d4', 'a1b2c3d', mode);

		expect(confirm).toHaveBeenCalledTimes(1);
		expect(lastConfirmation().body).toContain(wording);
		expect(lastConfirmation().danger).toBe(false);
		expect(api.reset).toHaveBeenCalledWith('a1b2c3d4', mode);
	});

	it.each<ResetMode>(['soft', 'mixed'])(
		'goes straight ahead on a %s reset with the warning off',
		async (mode) => {
			confirmHistoryRewrite = false;

			await actions.resetTo('a1b2c3d4', 'a1b2c3d', mode);

			expect(confirm).not.toHaveBeenCalled();
			expect(api.reset).toHaveBeenCalledWith('a1b2c3d4', mode);
		}
	);

	it('resets nothing when the confirmation is declined', async () => {
		confirm.mockResolvedValueOnce(false);

		await actions.resetTo('a1b2c3d4', 'a1b2c3d', 'hard');

		expect(api.reset).not.toHaveBeenCalled();
	});
});

describe('revertCommit', () => {
	it('reverts once confirmed', async () => {
		await actions.revertCommit('a1b2c3d4', 'a1b2c3d');

		expect(api.revert).toHaveBeenCalledWith('a1b2c3d4');
		expect(ok).toHaveBeenCalledWith('Reverted a1b2c3d', undefined);
	});

	it('reverts nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.revertCommit('a1b2c3d4', 'a1b2c3d');
		expect(api.revert).not.toHaveBeenCalled();
	});
});

describe('cherryPick', () => {
	it('does nothing, and asks nothing, for an empty selection', async () => {
		await actions.cherryPick([], []);

		expect(confirm).not.toHaveBeenCalled();
		expect(api.cherryPick).not.toHaveBeenCalled();
	});

	it('names the commit when there is one', async () => {
		await actions.cherryPick(['a1'], ['a1b2c3d']);

		expect(lastConfirmation().title).toBe('Cherry pick a1b2c3d');
		expect(lastConfirmation().body).toContain('This commit is');
		expect(api.cherryPick).toHaveBeenCalledWith(['a1']);
	});

	it('counts them when there are several', async () => {
		await actions.cherryPick(['a1', 'b2', 'c3'], ['a1b2c3d', 'b2c3d4e', 'c3d4e5f']);

		expect(lastConfirmation().title).toBe('Cherry pick 3 commits');
		expect(lastConfirmation().body).toContain('These commits are');
		expect(ok).toHaveBeenCalledWith('Cherry picked 3 commits', undefined);
	});

	it('picks nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.cherryPick(['a1'], ['a1b2c3d']);
		expect(api.cherryPick).not.toHaveBeenCalled();
	});
});

describe('rebaseOntoCommit', () => {
	it('warns that the rebase rewrites, and rebases', async () => {
		await actions.rebaseOntoCommit('a1b2c3d4', 'a1b2c3d');

		expect(lastConfirmation().danger).toBe(true);
		expect(lastConfirmation().body).toContain('force push');
		expect(api.rebaseOnto).toHaveBeenCalledWith('a1b2c3d4');
	});

	it('rebases nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.rebaseOntoCommit('a1b2c3d4', 'a1b2c3d');
		expect(api.rebaseOnto).not.toHaveBeenCalled();
	});
});

describe('rebaseRangeOnto', () => {
	const oldest = { id: 'a1b2c3d4', short: 'a1b2c3d', parents: ['p1'] };

	/**
	 * A root commit has no parent to bound the range with, and `git` would read
	 * the missing bound as "the whole branch" — which is not what was selected.
	 */
	it('refuses a selection whose oldest commit is a root', async () => {
		await actions.rebaseRangeOnto({ ...oldest, parents: [] }, 3, 'main');

		expect(confirm).not.toHaveBeenCalled();
		expect(api.rebaseOnto).not.toHaveBeenCalled();
		expect(failed).toHaveBeenCalledWith(
			'Could not rebase the selection',
			'the oldest commit selected has no parent, so there is no range to move'
		);
	});

	it('bounds the range at the oldest commit’s parent', async () => {
		await actions.rebaseRangeOnto(oldest, 3, 'main');

		expect(lastConfirmation().title).toBe('Rebase 3 commits onto main');
		expect(api.rebaseOnto).toHaveBeenCalledWith('main', 'a1b2c3d4^');
		expect(ok).toHaveBeenCalledWith('Rebased 3 onto main', undefined);
	});

	it('says "commit" for a range of one', async () => {
		await actions.rebaseRangeOnto(oldest, 1, 'main');
		expect(lastConfirmation().title).toBe('Rebase 1 commit onto main');
	});

	it('rebases nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.rebaseRangeOnto(oldest, 3, 'main');
		expect(api.rebaseOnto).not.toHaveBeenCalled();
	});
});

describe('checkoutCommit', () => {
	it('explains the detached HEAD before detaching', async () => {
		await actions.checkoutCommit('a1b2c3d4', 'a1b2c3d');

		expect(lastConfirmation().body).toContain('detached HEAD');
		expect(api.checkoutDetached).toHaveBeenCalledWith('a1b2c3d4');
	});

	it('checks out nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.checkoutCommit('a1b2c3d4', 'a1b2c3d');
		expect(api.checkoutDetached).not.toHaveBeenCalled();
	});
});

describe('renameBranch', () => {
	it('renames to the typed name', async () => {
		prompt.mockResolvedValueOnce('feature/renamed');

		await actions.renameBranch('feature/old');

		expect(api.renameBranch).toHaveBeenCalledWith('feature/old', 'feature/renamed');
		expect(ok).toHaveBeenCalledWith('Renamed feature/old to feature/renamed', undefined);
	});

	it('renames nothing when the prompt is dismissed', async () => {
		prompt.mockResolvedValueOnce(null);
		await actions.renameBranch('feature/old');
		expect(api.renameBranch).not.toHaveBeenCalled();
	});

	/** Confirming the pre-filled name unchanged is a no-op, not a rename to itself. */
	it('renames nothing when the name comes back unchanged', async () => {
		prompt.mockResolvedValueOnce('feature/old');
		await actions.renameBranch('feature/old');
		expect(api.renameBranch).not.toHaveBeenCalled();
	});
});

describe('deleteBranch', () => {
	it('is gentle and unforced for a merged branch', async () => {
		await actions.deleteBranch('feature/done', true);

		expect(lastConfirmation().danger).toBe(false);
		expect(lastConfirmation().body).toContain('nothing is lost');
		expect(api.deleteBranch).toHaveBeenCalledWith('feature/done', false);
	});

	/** Unmerged is the case where commits are actually lost, so it forces and says so. */
	it('is dangerous and forced for an unmerged branch', async () => {
		await actions.deleteBranch('feature/wip', false);

		expect(lastConfirmation().danger).toBe(true);
		expect(lastConfirmation().body).toContain('reflog');
		expect(api.deleteBranch).toHaveBeenCalledWith('feature/wip', true);
	});

	it('deletes nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.deleteBranch('feature/wip', false);
		expect(api.deleteBranch).not.toHaveBeenCalled();
	});
});

describe('deleteTag', () => {
	it('deletes the local tag once confirmed', async () => {
		await actions.deleteTag('v1.0.0');

		expect(lastConfirmation().danger).toBe(true);
		expect(api.deleteTag).toHaveBeenCalledWith('v1.0.0');
	});

	it('deletes nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.deleteTag('v1.0.0');
		expect(api.deleteTag).not.toHaveBeenCalled();
	});
});

describe('stash', () => {
	it.each<[StashAction, string, string, boolean]>([
		['apply', 'Apply', 'Applied', false],
		['pop', 'Pop', 'Popped', false],
		['drop', 'Drop', 'Dropped', true]
	])('words %s as its own operation', async (action, title, done, danger) => {
		await actions.stash(2, 'stash@{2}', action);

		expect(lastConfirmation().title).toBe(`${title} stash@{2}`);
		expect(lastConfirmation().confirmLabel).toBe(title);
		expect(lastConfirmation().danger).toBe(danger);
		expect(api.stashAction).toHaveBeenCalledWith(2, action);
		expect(ok).toHaveBeenCalledWith(`${done} stash@{2}`, undefined);
	});

	it('does nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.stash(0, 'stash@{0}', 'drop');
		expect(api.stashAction).not.toHaveBeenCalled();
	});

	it('reports git’s own refusal', async () => {
		const boom = new Error('could not restore untracked files');
		api.stashAction.mockRejectedValueOnce(boom);

		await actions.stash(0, 'stash@{0}', 'pop');

		expect(failed).toHaveBeenCalledWith('Could not pop stash@{0}', boom);
	});
});

/**
 * FEAT-018 changed the shape of these two. They start a worker and return, so
 * there is no outcome here to report — the notice comes from the done event,
 * which is `$lib/network/store.svelte`'s job and is tested there.
 *
 * What is still this module's job, and still asserted, is that neither asks a
 * question first: nothing is lost by fetching, and a push that would lose
 * something is refused by git rather than by a dialog we would have to keep in
 * step with git's rules.
 */
describe('remotes', () => {
	beforeEach(() => {
		startError = null;
		startFetch.mockResolvedValue(true);
		startPush.mockResolvedValue(true);
	});

	it('fetches without asking — nothing is lost by fetching', async () => {
		await actions.fetchAll();

		expect(confirm).not.toHaveBeenCalled();
		expect(startFetch).toHaveBeenCalledTimes(1);
	});

	it('pushes without asking', async () => {
		await actions.pushCurrent();

		expect(confirm).not.toHaveBeenCalled();
		expect(startPush).toHaveBeenCalledTimes(1);
	});

	it('reports a start that failed, which is the only outcome it sees', async () => {
		// A push git rejects fails on the done event, not here. This is the
		// other case: the worker could not be started at all.
		startPush.mockResolvedValueOnce(false);
		startError = 'a fetch or push is already running';

		await actions.pushCurrent();

		expect(failed).toHaveBeenCalledWith('Could not push', startError);
	});

	it('says nothing when a start was refused for no stated reason', async () => {
		startPush.mockResolvedValueOnce(false);

		await actions.pushCurrent();

		expect(failed).not.toHaveBeenCalled();
	});
});

/**
 * FEAT-038 — pull, and the uncommitted work in the way of it.
 *
 * Pulling onto a dirty working copy is where git's own refusal is correct and
 * useless: "Your local changes would be overwritten by merge" names the problem
 * and leaves the user to work out that the answer is a stash. The answer is
 * offered instead — and the ordering around a *failed* pull is the part that
 * could lose work, so it is the part most heavily asserted.
 */
describe('pull', () => {
	it('fast-forwards without asking, on a clean working copy', async () => {
		await actions.pull();

		expect(confirm).not.toHaveBeenCalled();
		expect(api.pull).toHaveBeenCalledWith('fastForwardOnly', '');
		expect(ok).toHaveBeenCalledWith('Pulled', undefined);
	});

	/** Merging and rebasing both write history and can stop in a conflict. */
	it.each(['merge', 'rebase'] as const)('asks before it %ss, even when clean', async (mode) => {
		await actions.pull(mode);

		expect(confirm).toHaveBeenCalledTimes(1);
		expect(api.pull).toHaveBeenCalledWith(mode, '');
	});

	it('marks a rebase pull as the destructive one', async () => {
		await actions.pull('rebase');
		expect(lastConfirmation().danger).toBe(true);

		await actions.pull('merge');
		expect(lastConfirmation().danger).toBe(false);
	});

	it('pulls nothing when the confirmation is declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.pull('merge');
		expect(api.pull).not.toHaveBeenCalled();
	});

	describe('with uncommitted changes', () => {
		it('asks first, naming how many there are', async () => {
			working = 3;
			await actions.pull();

			expect(confirm).toHaveBeenCalledTimes(1);
			expect(lastConfirmation().title).toContain('3 uncommitted changes');
		});

		it('says "change" for one of them', async () => {
			working = 1;
			await actions.pull();
			expect(lastConfirmation().title).toContain('1 uncommitted change');
		});

		it('stashes, pulls, then puts the changes back', async () => {
			working = 2;
			await actions.pull();

			expect(api.stashPush).toHaveBeenCalledWith('Before pull', true);
			expect(api.pull).toHaveBeenCalledWith('fastForwardOnly', '');
			expect(api.stashAction).toHaveBeenCalledWith(0, 'pop');
		});

		it('does none of it when declined', async () => {
			working = 2;
			confirm.mockResolvedValueOnce(false);
			await actions.pull();

			expect(api.stashPush).not.toHaveBeenCalled();
			expect(api.pull).not.toHaveBeenCalled();
		});

		/** Nothing has been touched yet, so nothing needs putting back. */
		it('does not pull when the stash itself fails', async () => {
			working = 2;
			api.stashPush.mockRejectedValueOnce(new Error('cannot stash'));

			await actions.pull();

			expect(api.pull).not.toHaveBeenCalled();
			expect(api.stashAction).not.toHaveBeenCalled();
			expect(failed).toHaveBeenCalledWith(
				'Could not stash your changes, so nothing was pulled',
				expect.anything()
			);
		});

		/**
		 * The one that could lose work. Restoring on top of a half-finished pull
		 * would hand back a working copy in a state neither the user nor git put
		 * it in — so the changes stay in the stash, and the message says so.
		 */
		it('leaves the changes in the stash when the pull fails', async () => {
			working = 2;
			api.pull.mockRejectedValueOnce(new Error('would be overwritten'));

			await actions.pull();

			expect(api.stashAction).not.toHaveBeenCalled();
			expect(failed).toHaveBeenCalledWith(
				'Could not pull — your changes are in the stash',
				expect.anything()
			);
		});

		/** Pulled but not restored is a different situation, and says which. */
		it('distinguishes a failed restore from a failed pull', async () => {
			working = 2;
			api.stashAction.mockRejectedValueOnce(new Error('conflict'));

			await actions.pull();

			expect(api.pull).toHaveBeenCalled();
			expect(failed).toHaveBeenCalledWith(
				'Pulled, but could not put your changes back — they are in the stash',
				expect.anything()
			);
		});

		it('re-reads afterwards, whichever way it went', async () => {
			working = 2;
			await actions.pull();
			expect(repoRefresh).toHaveBeenCalled();

			vi.clearAllMocks();
			working = 2;
			confirm.mockResolvedValue(true);
			api.pull.mockRejectedValueOnce(new Error('nope'));
			await actions.pull();
			expect(repoRefresh).toHaveBeenCalled();
		});
	});
});

/** BUG-055: the bottom bar's Stash puts the work away here, in one question. */
describe('quickStash', () => {
	it('asks with WIP on the branch as the message, then stashes everything', async () => {
		working = 3;
		prompt.mockResolvedValueOnce('WIP on feature/tabs: half the drag');

		expect(await actions.quickStash()).toBe(true);

		const asked = prompt.mock.calls[0][0];
		expect(asked.value).toBe('WIP on feature/tabs');
		expect(asked.confirmLabel).toBe('Stash now');
		expect(api.stashPush).toHaveBeenCalledWith('WIP on feature/tabs: half the drag', true);
		working = 0;
	});

	it('stashes nothing when refused', async () => {
		working = 2;
		prompt.mockResolvedValueOnce(null);

		expect(await actions.quickStash()).toBe(false);
		expect(api.stashPush).not.toHaveBeenCalled();
		working = 0;
	});

	it('asks nothing when there is nothing to stash', async () => {
		working = 0;

		expect(await actions.quickStash()).toBe(false);
		expect(prompt).not.toHaveBeenCalled();
		expect(ok).toHaveBeenCalledWith('Nothing to stash', undefined);
	});
});
