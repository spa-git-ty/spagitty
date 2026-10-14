// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The blocks a recipe is made of (FEAT-133).
 *
 * Closed: these eighteen are all there are, and adding one is a change to
 * Spagitty, not to a recipe. The list is `recipes::CATALOGUE` in the core,
 * which is the one that decides what can run; a Rust test reads this file
 * and fails if the two have come apart.
 *
 * There is no block that forces a push, deletes an unmerged branch or resets
 * hard, here or there.
 */

import type { BlockId, Step, World } from './types';

export type GroupId = 'save' | 'sync' | 'combine' | 'branch' | 'shape' | 'guard';

export interface Group {
	id: GroupId;
	label: string;
	/**
	 * The colour of the small square beside its blocks. Muted on purpose: the
	 * author asked for a tool for professionals, and a wall of bright bricks
	 * is a toy. The blocks themselves are neutral cards.
	 */
	tone: string;
}

export const GROUPS: Group[] = [
	{ id: 'save', label: 'Save', tone: 'var(--warn)' },
	{ id: 'sync', label: 'Sync', tone: 'var(--lane-5)' },
	{ id: 'combine', label: 'Combine', tone: 'var(--lane-4)' },
	{ id: 'branch', label: 'Branch', tone: 'var(--ok)' },
	{ id: 'shape', label: 'Shape history', tone: 'var(--danger)' },
	{ id: 'guard', label: 'Guard', tone: 'var(--muted)' }
];

export interface Block {
	id: BlockId;
	label: string;
	group: GroupId;
	/** The one key that adds it. */
	key: string;
	/** What it does, for its hover. */
	what: string;
}

export const CATALOGUE: Block[] = [
	{ id: 'stash', label: 'Stash', group: 'save', key: 'S', what: 'Put the changes in the working tree away' },
	{ id: 'pop', label: 'Pop stash', group: 'save', key: 'P', what: 'Bring back the newest stash a Stash above made' },
	{ id: 'commit', label: 'Commit', group: 'save', key: 'C', what: 'Commit everything, or what is staged' },
	{ id: 'amend', label: 'Amend', group: 'save', key: 'A', what: 'Add what is staged to the last commit' },
	{ id: 'fetch', label: 'Fetch', group: 'sync', key: 'F', what: 'Fetch every remote, or one' },
	{ id: 'pull', label: 'Pull', group: 'sync', key: 'L', what: 'Fetch and bring the upstream in' },
	{ id: 'push', label: 'Push', group: 'sync', key: 'U', what: 'Push the branch. Never forced' },
	{ id: 'merge', label: 'Merge', group: 'combine', key: 'M', what: 'Merge another branch into this one' },
	{ id: 'rebase', label: 'Rebase', group: 'combine', key: 'R', what: 'Replay this branch onto another' },
	{ id: 'cherryPick', label: 'Cherry-pick', group: 'combine', key: 'Y', what: 'Apply chosen commits here' },
	{ id: 'switch', label: 'Switch', group: 'branch', key: 'W', what: 'Check another branch out' },
	{ id: 'newBranch', label: 'New branch', group: 'branch', key: 'N', what: 'Make a branch and switch to it' },
	{ id: 'deleteBranch', label: 'Delete branch', group: 'branch', key: 'D', what: 'Delete a branch, only if it is merged' },
	{ id: 'squash', label: 'Squash', group: 'shape', key: 'Q', what: 'Make the last commits one' },
	{ id: 'reword', label: 'Reword', group: 'shape', key: 'E', what: 'Change the last commit’s message' },
	{ id: 'check', label: 'Run checks', group: 'guard', key: 'T', what: 'Run one of this repository’s checks, and stop if it fails' },
	{ id: 'ask', label: 'Stop and ask me', group: 'guard', key: '?', what: 'Stop here, at any level' },
	{ id: 'clean', label: 'Stop unless clean', group: 'guard', key: 'G', what: 'Stop if the working tree has changes' }
];

export function blockOf(id: BlockId): Block {
	return CATALOGUE.find((block) => block.id === id) as Block;
}

export function groupOf(id: BlockId): Group {
	const group = blockOf(id).group;
	return GROUPS.find((each) => each.id === group) as Group;
}

/** The block a key adds, whichever case it was typed in. */
export function byKey(key: string): Block | null {
	const wanted = key.length === 1 ? key.toUpperCase() : key;
	return CATALOGUE.find((block) => block.key === wanted) ?? null;
}

/**
 * A block as it is when it is first added: what can be chosen for it already
 * chosen, from what the repository has, and what cannot left for the person.
 */
export function fresh(id: BlockId, world: World | null): Step {
	const remote = world?.remotes.includes('origin') ? 'origin' : (world?.remotes[0] ?? '');
	switch (id) {
		case 'stash':
			return { block: 'stash', params: { untracked: true } };
		case 'pop':
			return { block: 'pop', params: {} };
		case 'commit':
			return { block: 'commit', params: { all: true, message: { kind: 'text', text: '' } } };
		case 'amend':
			return { block: 'amend', params: { message: null } };
		case 'fetch':
			return { block: 'fetch', params: { remote: '' } };
		case 'pull':
			return { block: 'pull', params: { remote, mode: 'fastForwardOnly' } };
		case 'push':
			return { block: 'push', params: { remote } };
		case 'merge':
			return { block: 'merge', params: { branch: '', how: 'commit' } };
		case 'rebase':
			return { block: 'rebase', params: { onto: '' } };
		case 'cherryPick':
			return { block: 'cherryPick', params: { commits: [] } };
		case 'switch':
			return { block: 'switch', params: { branch: '' } };
		case 'newBranch':
			return { block: 'newBranch', params: { name: '', from: '' } };
		case 'deleteBranch':
			return { block: 'deleteBranch', params: { branch: '' } };
		case 'squash':
			return { block: 'squash', params: { count: 2, message: { kind: 'text', text: '' } } };
		case 'reword':
			return { block: 'reword', params: { message: { kind: 'text', text: '' } } };
		case 'check':
			return { block: 'check', params: { command: world?.checks[0] ?? '' } };
		case 'ask':
			return { block: 'ask', params: {} };
		case 'clean':
			return { block: 'clean', params: {} };
	}
}
