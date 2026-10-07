// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * What the graph's menus actually do.
 *
 * The menus in `CommitRows` describe operations; this module performs them.
 * They are kept apart because the sentence shown before a `reset --hard` is as
 * much a part of the operation as the command itself, and burying it in a
 * component's markup means the next screen that offers the same operation
 * writes a different sentence — or none.
 *
 * Every function here follows the same three steps:
 *
 * 1. **Ask, when there is something to lose.** Losing operations always ask,
 *    whatever the Settings toggle says. `confirmHistoryRewrite` governs the
 *    rewrites that *move* commits rather than destroying them — a rebase can be
 *    undone from the reflog, a `reset --hard` of uncommitted work cannot.
 * 2. **Run it**, and let git's own refusal be the error text.
 * 3. **Re-read.** The filesystem watcher will notice too, but a graph that
 *    updates when the watcher gets round to it feels like a graph that did not
 *    do anything.
 *
 * Nothing here throws. A failed operation is a notice, because the caller is a
 * menu entry and a menu entry has nowhere to catch.
 */

import * as api from '../api';
import { graph } from './store.svelte';
import { repo } from '../repo.svelte';
import { settings } from '../settings/store.svelte';
import { deleteBody } from '$lib/branches/actions';
import { network } from '$lib/network/store.svelte';
import { delight } from '$lib/delight/store.svelte';
import { cherryPicked, merged, recovered, switchedBranch } from '$lib/delight/watch';
import { dialog } from '../ui/dialog.svelte';
import { notice } from '../ui/notice.svelte';
import type { Integration, PullMode, ResetMode, StashAction } from '../types';

/** Re-read HEAD, the counts and the walk. */
async function refresh(): Promise<void> {
	await repo.refresh();
	await graph.reload();
}

/**
 * Run `work`, report either way, and refresh on success.
 *
 * `success` is written in the past tense and names what changed, because it is
 * read after the fact: "Reset to a1b2c3d", not "Resetting…".
 *
 * Answers whether the repository changed. Most callers are menu entries with
 * nothing to do with the answer and ignore it; a screen holding its own list of
 * what was just written needs it, so it can re-read exactly when there is
 * something new to read.
 */
async function perform(
	success: string,
	failure: string,
	work: () => Promise<unknown>
): Promise<boolean> {
	try {
		await work();
		notice.ok(success);
		await refresh();
		return true;
	} catch (error) {
		notice.failed(failure, error);
		return false;
	}
}

/** Whether the user asked to be warned before history is rewritten. */
function warnsOnRewrite(): boolean {
	return settings.settings.confirmHistoryRewrite;
}

// --- Commits ---------------------------------------------------------------

/** Put a commit id on the clipboard. The full id, because that is what pastes usefully. */
export async function copyId(id: string, short: string): Promise<void> {
	try {
		await navigator.clipboard.writeText(id);
		notice.ok(`Copied ${short}`);
	} catch (error) {
		// A webview can refuse clipboard access outright, and silently failing to
		// copy is the kind of thing people only notice after pasting.
		notice.failed('Could not copy the commit id', error);
	}
}

/**
 * Create a branch at a commit and check it out, with the name already typed —
 * the graph's own name field, opened by the toolbar's Branch (FEAT-104).
 */
export function createBranchNamed(name: string, id: string): Promise<boolean> {
	return perform(`Created ${name}`, 'Could not create the branch', () =>
		api.createBranch(name, id, true)
	);
}

/**
 * The bottom bar's Stash (BUG-055): put the work in progress away, here, in
 * one question — not the Stash screen. The message starts as `WIP on
 * <branch>` and can be changed; untracked files go too, so the working copy
 * is left clean. With nothing to stash, it says so and asks nothing.
 */
export async function quickStash(): Promise<boolean> {
	const changed = repo.counts.working ?? 0;
	if (changed === 0) {
		notice.ok('Nothing to stash');
		return false;
	}
	const branch = repo.info?.head.branch ?? repo.info?.head.short ?? 'HEAD';
	const message = await dialog.prompt({
		title: 'Stash your changes',
		body: `${changed} changed ${changed === 1 ? 'file is' : 'files are'} put away, untracked ones too, and the working copy is left clean.`,
		label: 'Message',
		value: `WIP on ${branch}`,
		confirmLabel: 'Stash now'
	});
	if (message === null) return false;
	return perform('Stashed', 'Could not stash', () => api.stashPush(message, true));
}

export async function createTagAt(id: string, short: string): Promise<void> {
	const name = await dialog.prompt({
		title: 'Create tag here',
		body: `The tag will point at ${short}.`,
		label: 'Tag name',
		placeholder: 'v1.0.0',
		confirmLabel: 'Create'
	});
	if (name === null) return;

	await perform(`Tagged ${short} as ${name}`, 'Could not create the tag', () =>
		api.createTag(name, id)
	);
}

/** What each reset mode does, in the words the confirmation uses. */
const RESET_BODY: Record<ResetMode, (short: string) => string> = {
	soft: (short) =>
		`The branch moves to ${short}. Everything after it stays, staged and ready to commit again.`,
	mixed: (short) =>
		`The branch moves to ${short}. Everything after it stays in your files, unstaged.`,
	hard: (short) =>
		`The branch moves to ${short} and your files are put back to match it. ` +
		'Uncommitted changes are lost and cannot be recovered.'
};

export async function resetTo(id: string, short: string, mode: ResetMode): Promise<void> {
	// `hard` always asks. The other two move a branch, which the reflog can undo.
	if (mode === 'hard' || warnsOnRewrite()) {
		const agreed = await dialog.confirm({
			title: `Reset ${mode} to ${short}`,
			body: RESET_BODY[mode](short),
			confirmLabel: 'Reset',
			danger: mode === 'hard'
		});
		if (!agreed) return;
	}

	await perform(`Reset ${mode} to ${short}`, 'Could not reset', () => api.reset(id, mode));
}

export async function revertCommit(id: string, short: string): Promise<void> {
	const agreed = await dialog.confirm({
		title: `Revert ${short}`,
		body: 'A new commit is added that undoes this one. Nothing already in history is changed.',
		confirmLabel: 'Revert'
	});
	if (!agreed) return;

	await perform(`Reverted ${short}`, 'Could not revert', () => api.revert(id));
}

/**
 * Replay commits onto the current branch.
 *
 * `ids` is oldest-first, which is the order they will be applied in and the
 * order they were written in — the order most likely to apply cleanly.
 */
export async function cherryPick(ids: string[], labels: string[]): Promise<void> {
	if (ids.length === 0) return;

	const what = ids.length === 1 ? labels[0] : `${ids.length} commits`;
	const agreed = await dialog.confirm({
		title: `Cherry pick ${what}`,
		body:
			`${ids.length === 1 ? 'This commit is' : 'These commits are'} copied onto the ` +
			'branch you have checked out, oldest first. The originals stay where they are.',
		confirmLabel: 'Cherry pick'
	});
	if (!agreed) return;

	const done = await perform(`Cherry picked ${what}`, 'Could not cherry pick', () =>
		api.cherryPick(ids)
	);
	if (done) cherryPicked(ids.length);
}

/**
 * Replay the current branch onto a commit that may have no branch of its own.
 *
 * This is the graph's answer to "I want to be based on that point" without
 * having to create a throwaway branch there first.
 */
export async function rebaseOntoCommit(id: string, short: string): Promise<void> {
	const agreed = await dialog.confirm({
		title: `Rebase onto ${short}`,
		body:
			'Your current branch is replayed on top of that commit. Its commits are rewritten, ' +
			'so anything already pushed will need a force push.',
		confirmLabel: 'Rebase',
		danger: true
	});
	if (!agreed) return;

	await perform(`Rebased onto ${short}`, 'Could not rebase', () => api.rebaseOnto(id));
}

/**
 * Move a run of commits onto a branch.
 *
 * `oldest` is the oldest commit of the selection; its parent is the upstream,
 * so the range that moves is exactly what was selected. A selection whose
 * oldest commit is a root has no parent to bound the range and is refused —
 * git would take it to mean the whole branch, which is not what was selected.
 */
export async function rebaseRangeOnto(
	oldest: { id: string; short: string; parents: string[] },
	count: number,
	target: string
): Promise<void> {
	if (oldest.parents.length === 0) {
		notice.failed(
			'Could not rebase the selection',
			'the oldest commit selected has no parent, so there is no range to move'
		);
		return;
	}

	const agreed = await dialog.confirm({
		title: `Rebase ${count} ${count === 1 ? 'commit' : 'commits'} onto ${target}`,
		body:
			`The run starting at ${oldest.short} is replayed on top of ${target}. ` +
			'Those commits are rewritten, so anything already pushed will need a force push.',
		confirmLabel: 'Rebase',
		danger: true
	});
	if (!agreed) return;

	await perform(
		`Rebased ${count} onto ${target}`,
		'Could not rebase the selection',
		// `<oldest>^` is the parent: everything after it, up to HEAD, moves.
		() => api.rebaseOnto(target, `${oldest.id}^`)
	);
}

export async function checkoutCommit(id: string, short: string): Promise<void> {
	const agreed = await dialog.confirm({
		title: `Check out ${short}`,
		body:
			'You will be on no branch — a detached HEAD. Commits made here belong to nothing ' +
			'until you create a branch for them.',
		confirmLabel: 'Check out'
	});
	if (!agreed) return;

	await perform(`Checked out ${short}`, 'Could not check out that commit', () =>
		api.checkoutDetached(id)
	);
}

/**
 * Whether HEAD is detached right now.
 *
 * Read *before* a checkout rather than after, because the interesting fact is
 * where the user was coming from: checking out a branch while detached is the
 * escape, and afterwards there is nothing left to say it happened (FEAT-072).
 */
function detached(): boolean {
	return repo.info?.head.detached ?? false;
}

// --- Branches --------------------------------------------------------------

export async function checkoutBranch(name: string): Promise<void> {
	const wasDetached = detached();
	const done = await perform(`Switched to ${name}`, `Could not switch to ${name}`, () =>
		api.checkout(name)
	);
	if (!done) return;

	switchedBranch();
	if (wasDetached) recovered('detached');
}

/** What each way of integrating one branch into another does, for the menu. */
export const INTEGRATIONS: { how: Integration; label: string; body: (s: string, t: string) => string }[] =
	[
		{
			how: 'merge',
			label: 'Merge',
			body: (source, target) =>
				`${source} is merged into ${target}. Git fast-forwards if it can, and writes a merge commit if it cannot.`
		},
		{
			how: 'mergeNoFastForward',
			label: 'Merge, always commit',
			body: (source, target) =>
				`${source} is merged into ${target} with a merge commit, even where a fast-forward was possible. The branch stays visible in the graph.`
		},
		{
			how: 'fastForward',
			label: 'Fast-forward',
			body: (source, target) =>
				`${target} moves up to ${source}. No commit is written, and git refuses rather than merging if it cannot.`
		},
		{
			how: 'rebase',
			label: 'Rebase',
			body: (source, target) =>
				`${target} is replayed on top of ${source}. Its commits are rewritten, so anything already pushed will need a force push.`
		}
	];

export async function integrate(source: string, target: string, how: Integration): Promise<void> {
	const entry = INTEGRATIONS.find((candidate) => candidate.how === how);
	if (!entry) return;

	const rewrites = how === 'rebase';
	if (rewrites || warnsOnRewrite()) {
		const agreed = await dialog.confirm({
			title: `${entry.label} ${source} into ${target}`,
			body: entry.body(source, target),
			confirmLabel: entry.label,
			danger: rewrites
		});
		if (!agreed) return;
	}

	const done = await perform(
		`${entry.label}d ${source} into ${target}`,
		`Could not ${entry.label.toLowerCase()}`,
		() => api.integrate(source, how)
	);
	if (!done) return;

	// A rebase integration is a rebase, not a merge. Counting it as one would
	// let Octopus be earned without ever folding two histories together.
	if (how === 'rebase') delight.record({ kind: 'rebase', commits: 0, conflicts: 0, interactive: false });
	else merged(how === 'fastForward');
}

export async function renameBranch(name: string): Promise<void> {
	const next = await dialog.prompt({
		title: `Rename ${name}`,
		body: 'Only the local branch is renamed. A remote that already has it keeps the old name.',
		label: 'New name',
		value: name,
		confirmLabel: 'Rename'
	});
	if (next === null || next === name) return;

	await perform(`Renamed ${name} to ${next}`, 'Could not rename the branch', () =>
		api.renameBranch(name, next)
	);
}

/**
 * Delete a local branch.
 *
 * `merged` decides both the wording and whether git is asked to force. An
 * unmerged branch is the case where commits are actually lost, and it says so.
 */
export async function deleteBranch(name: string, merged: boolean): Promise<void> {
	const agreed = await dialog.confirm({
		title: `Delete ${name}`,
		// One sentence, shared with the Branches screen (FEAT-013), so the two
		// places that delete a branch cannot come to say different things about
		// what it costs.
		body: deleteBody(name, merged),
		confirmLabel: 'Delete',
		danger: !merged
	});
	if (!agreed) return;

	await perform(`Deleted ${name}`, 'Could not delete the branch', () =>
		api.deleteBranch(name, !merged)
	);
}

export async function deleteTag(name: string): Promise<void> {
	const agreed = await dialog.confirm({
		title: `Delete ${name}`,
		body: 'Only the local tag is removed. A remote that has it keeps it until it is deleted there too.',
		confirmLabel: 'Delete',
		danger: true
	});
	if (!agreed) return;

	await perform(`Deleted ${name}`, 'Could not delete the tag', () => api.deleteTag(name));
}

// --- Stashes ---------------------------------------------------------------

const STASH_WORDING: Record<StashAction, { title: string; body: string; done: string }> = {
	apply: {
		title: 'Apply',
		body: 'The stashed changes are put back into your files and the entry is kept.',
		done: 'Applied'
	},
	pop: {
		title: 'Pop',
		body: 'The stashed changes are put back into your files and the entry is removed.',
		done: 'Popped'
	},
	drop: {
		title: 'Drop',
		body: 'The entry is removed without being restored. It survives in the reflog until git expires it.',
		done: 'Dropped'
	}
};

/**
 * Answers whether the stash list changed, because the Stash screen holds its
 * own copy of it and has to re-read when — and only when — it did.
 */
export async function stash(
	index: number,
	name: string,
	action: StashAction
): Promise<boolean> {
	const wording = STASH_WORDING[action];
	const agreed = await dialog.confirm({
		title: `${wording.title} ${name}`,
		body: wording.body,
		confirmLabel: wording.title,
		danger: action === 'drop'
	});
	if (!agreed) return false;

	return await perform(`${wording.done} ${name}`, `Could not ${action} ${name}`, () =>
		api.stashAction(index, action)
	);
}

// --- Remotes ---------------------------------------------------------------

/** What each way of pulling does, in the words the confirmation uses. */
const PULL_WORDING: Record<PullMode, { title: string; body: string; done: string }> = {
	fastForwardOnly: {
		title: 'Pull',
		body: 'Your branch moves forward to match the remote. If it has commits of its own, nothing happens and git says so — this can never write a merge commit or leave a conflict.',
		done: 'Pulled'
	},
	merge: {
		title: 'Pull and merge',
		body: 'Your branch moves forward where it can, and a merge commit is written where it cannot. A conflict here stops in the working copy for you to resolve.',
		done: 'Pulled'
	},
	rebase: {
		title: 'Pull and rebase',
		body: 'Your own commits are replayed on top of the remote’s. They are rewritten, so anything already pushed will need a force push.',
		done: 'Pulled and rebased'
	}
};

/**
 * Pull, and deal with uncommitted work first.
 *
 * Pulling onto a dirty working copy is the case where git's own refusal —
 * "Your local changes would be overwritten by merge" — is technically correct
 * and useless: it names the problem and leaves the user to work out that the
 * answer is a stash. So the answer is offered.
 *
 * The stash is popped afterwards **only if the pull succeeded**. A failed pull
 * that then restored the changes on top would hand back a working copy in a
 * state neither the user nor git put it in.
 */
export async function pull(mode: PullMode = 'fastForwardOnly'): Promise<void> {
	const wording = PULL_WORDING[mode];
	const dirty = repo.counts.working ?? 0;

	if (dirty > 0) {
		const agreed = await dialog.confirm({
			title: `${wording.title} with ${dirty} uncommitted ${dirty === 1 ? 'change' : 'changes'}`,
			body:
				`${wording.body}\n\n` +
				'Your uncommitted changes are stashed first and put back afterwards. If the pull ' +
				'fails they stay in the stash, where nothing has been lost.',
			confirmLabel: 'Stash and pull',
			danger: mode === 'rebase'
		});
		if (!agreed) return;

		try {
			await api.stashPush('Before pull', true);
		} catch (error) {
			notice.failed('Could not stash your changes, so nothing was pulled', error);
			return;
		}

		try {
			await api.pull(mode, '');
		} catch (error) {
			notice.failed('Could not pull — your changes are in the stash', error);
			await refresh();
			return;
		}

		try {
			await api.stashAction(0, 'pop');
			notice.ok(`${wording.done}, and your changes are back`);
		} catch (error) {
			// The pull worked; only the restore did not. Say which, because the
			// two have very different next steps.
			notice.failed('Pulled, but could not put your changes back — they are in the stash', error);
		}

		await refresh();
		return;
	}

	if (mode !== 'fastForwardOnly') {
		const agreed = await dialog.confirm({
			title: wording.title,
			body: wording.body,
			confirmLabel: wording.title,
			danger: mode === 'rebase'
		});
		if (!agreed) return;
	}

	await perform(wording.done, 'Could not pull', () => api.pull(mode, ''));
}

/**
 * Fetch every remote.
 *
 * Not through `perform`: since FEAT-018 this starts a worker and returns, so
 * there is no outcome to report yet. The notice comes from the done event, and
 * what happens in between is on the toolbar.
 */
export async function fetchAll(): Promise<void> {
	if (!(await network.fetch())) {
		if (network.error) notice.failed('Could not fetch', network.error);
	}
}

/** Push the current branch. Starts a worker, the same as `fetchAll`. */
export async function pushCurrent(): Promise<void> {
	if (!(await network.push())) {
		if (network.error) notice.failed('Could not push', network.error);
	}
}
