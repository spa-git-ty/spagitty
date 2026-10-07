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
import type { Integration, ResetMode, StashAction } from '$lib/types';

/** The options object every action hands to the dialog, as far as tests care. */
interface Asked {
	title: string;
	body: string;
	confirmLabel: string;
	danger?: boolean;
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
	integrate: vi.fn(() => Promise.resolve()),
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

describe('integrate', () => {
	it('has wording for every integration it offers', () => {
		expect(actions.INTEGRATIONS.map((entry) => entry.how)).toEqual([
			'merge',
			'mergeNoFastForward',
			'fastForward',
			'rebase'
		]);
	});

	it('does nothing for an integration it does not know', async () => {
		await actions.integrate('feature/x', 'main', 'nonsense' as Integration);

		expect(confirm).not.toHaveBeenCalled();
		expect(api.integrate).not.toHaveBeenCalled();
	});

	it('always asks before a rebase, even with the warning off', async () => {
		confirmHistoryRewrite = false;

		await actions.integrate('feature/x', 'main', 'rebase');

		expect(confirm).toHaveBeenCalledTimes(1);
		expect(lastConfirmation().danger).toBe(true);
		expect(api.integrate).toHaveBeenCalledWith('feature/x', 'rebase');
	});

	it.each<Integration>(['merge', 'mergeNoFastForward', 'fastForward'])(
		'asks before %s while the warning is on',
		async (how) => {
			await actions.integrate('feature/x', 'main', how);

			expect(confirm).toHaveBeenCalledTimes(1);
			expect(lastConfirmation().danger).toBe(false);
			expect(api.integrate).toHaveBeenCalledWith('feature/x', how);
		}
	);

	it('goes straight ahead on a merge with the warning off', async () => {
		confirmHistoryRewrite = false;

		await actions.integrate('feature/x', 'main', 'merge');

		expect(confirm).not.toHaveBeenCalled();
		expect(api.integrate).toHaveBeenCalledWith('feature/x', 'merge');
	});

	it('integrates nothing when declined', async () => {
		confirm.mockResolvedValueOnce(false);
		await actions.integrate('feature/x', 'main', 'merge');
		expect(api.integrate).not.toHaveBeenCalled();
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
