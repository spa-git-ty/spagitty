// SPDX-License-Identifier: GPL-3.0-or-later

import { describe, expect, it } from 'vitest';
import { conflicted, error, runOf, SQUASH, SYNC, warning } from '../../testing/recipe-fixtures';
import { byKey, CATALOGUE, fresh, GROUPS, blockOf, groupOf } from './catalogue';
import {
	added,
	blank,
	choiceLabel,
	findingsAt,
	LEVELS,
	levelWords,
	mainChoice,
	moved,
	nextName,
	removed,
	replaced,
	runLabel,
	runShare,
	runStatus,
	same,
	took,
	withoutAgent
} from './model';
import type { Recipe, Step, Stop } from './types';

const ASK: Step = { block: 'ask', params: {} };

describe('the catalogue', () => {
	it('is the eighteen blocks in six groups', () => {
		expect(CATALOGUE).toHaveLength(18);
		expect(GROUPS.map((group) => CATALOGUE.filter((block) => block.group === group.id).length)).toEqual([
			4, 3, 3, 3, 2, 3
		]);
	});

	it('gives every block a key of its own', () => {
		const keys = CATALOGUE.map((block) => block.key);
		expect(new Set(keys).size).toBe(18);
		expect(keys.join('')).toBe('SPCAFLUMRYWNDQET?G');
	});

	it('finds a block by its key in either case', () => {
		expect(byKey('s')?.id).toBe('stash');
		expect(byKey('S')?.id).toBe('stash');
		expect(byKey('?')?.id).toBe('ask');
		expect(byKey('z')).toBeNull();
		expect(byKey('Enter')).toBeNull();
	});

	/** No block forces, deletes unmerged or resets hard, here as in the core. */
	it('has no block for a force push, a hard reset or a clean', () => {
		const ids = CATALOGUE.map((block) => block.id.toLowerCase());
		for (const never of ['force', 'reset', 'hard']) {
			expect(ids.some((id) => id.includes(never))).toBe(false);
		}
		expect(blockOf('push').what).toContain('Never forced');
		expect(blockOf('deleteBranch').what).toContain('only if it is merged');
	});

	it('knows a block’s group and its muted square', () => {
		expect(groupOf('merge').label).toBe('Combine');
		expect(groupOf('squash').label).toBe('Shape history');
		expect(GROUPS.every((group) => group.tone.startsWith('var(--'))).toBe(true);
	});

	it('starts a block with what the repository has already chosen', () => {
		const world = {
			branches: ['main'],
			remotes: ['fork', 'origin'],
			current: 'main',
			dirty: false,
			operation: null,
			upstream: false,
			unpushed: 0,
			rewritesPushed: [],
			protected: [],
			checks: ['cargo test'],
			agents: [],
			highest: 'signOff' as const
		};

		expect(fresh('push', world)).toEqual({ block: 'push', params: { remote: 'origin' } });
		expect(fresh('push', { ...world, remotes: ['fork'] })).toEqual({ block: 'push', params: { remote: 'fork' } });
		expect(fresh('push', null)).toEqual({ block: 'push', params: { remote: '' } });
		expect(fresh('check', world)).toEqual({ block: 'check', params: { command: 'cargo test' } });
		expect(fresh('stash', world)).toEqual({ block: 'stash', params: { untracked: true } });
		// What cannot be guessed is left for the person.
		expect(fresh('merge', world)).toEqual({ block: 'merge', params: { branch: '', how: 'commit' } });
		expect(fresh('commit', world)).toEqual({
			block: 'commit',
			params: { all: true, message: { kind: 'text', text: '' } }
		});
		// One of each, and each is its own block.
		for (const block of CATALOGUE) expect(fresh(block.id, world).block).toBe(block.id);
	});
});

describe('editing a stack', () => {
	const two: Recipe = { ...blank('r'), steps: [SYNC.steps[0], SYNC.steps[1]] };

	it('starts empty, at Ask every step, with the agent’s help off', () => {
		expect(blank('recipe-1')).toEqual({
			v: 1,
			name: 'recipe-1',
			branch: '',
			steps: [],
			level: 'stepByStep',
			agentHelp: false,
			agent: null,
			note: ''
		});
	});

	it('names a new recipe with the first number nothing has', () => {
		expect(nextName([])).toBe('recipe-1');
		expect(nextName(['recipe-1', 'sync', 'recipe-2'])).toBe('recipe-3');
		expect(nextName(['recipe-2'])).toBe('recipe-1');
	});

	it('adds at the end, removes one, and replaces one', () => {
		expect(added(two, ASK).steps.map((step) => step.block)).toEqual(['stash', 'pull', 'ask']);
		expect(removed(two, 0).steps.map((step) => step.block)).toEqual(['pull']);
		expect(replaced(two, 1, ASK).steps.map((step) => step.block)).toEqual(['stash', 'ask']);
		// The recipe that was is not changed.
		expect(two.steps).toHaveLength(2);
	});

	it('moves a step one place, and not past an end', () => {
		expect(moved(two, 1, -1).steps.map((step) => step.block)).toEqual(['pull', 'stash']);
		expect(moved(two, 0, 1).steps.map((step) => step.block)).toEqual(['pull', 'stash']);
		expect(moved(two, 0, -1)).toBe(two);
		expect(moved(two, 1, 1)).toBe(two);
		expect(moved(two, 5, -1)).toBe(two);
	});

	it('tells a recipe that changed from one that did not', () => {
		expect(same(two, { ...two, steps: [...two.steps] })).toBe(true);
		expect(same(two, { ...two, note: 'x' })).toBe(false);
		expect(same(two, null)).toBe(false);
		expect(same(null, null)).toBe(true);
	});

	/** The author's note: off, a message is typed. */
	it('switching the agent off turns every message it would write into one to type', () => {
		const helped: Recipe = {
			...SQUASH,
			steps: [
				{ block: 'commit', params: { all: true, message: { kind: 'agent' } } },
				{ block: 'squash', params: { count: 3, message: { kind: 'agent' } } },
				{ block: 'reword', params: { message: { kind: 'agent' } } },
				{ block: 'amend', params: { message: { kind: 'agent' } } },
				{ block: 'amend', params: { message: null } },
				{ block: 'commit', params: { all: false, message: { kind: 'text', text: 'Mine' } } },
				{ block: 'push', params: { remote: 'origin' } }
			]
		};

		const off = withoutAgent(helped);

		expect(off.agentHelp).toBe(false);
		expect(off.steps).toEqual([
			{ block: 'commit', params: { all: true, message: { kind: 'text', text: '' } } },
			{ block: 'squash', params: { count: 3, message: { kind: 'text', text: '' } } },
			{ block: 'reword', params: { message: { kind: 'text', text: '' } } },
			{ block: 'amend', params: { message: { kind: 'text', text: '' } } },
			{ block: 'amend', params: { message: null } },
			{ block: 'commit', params: { all: false, message: { kind: 'text', text: 'Mine' } } },
			{ block: 'push', params: { remote: 'origin' } }
		]);
		// The note and the agent are kept for when it is on again.
		expect(off.note).toBe('Keep it short.');
		expect(off.agent).toBe('claude');
	});
});

describe('the words', () => {
	it('names the four levels as 2.0 does, and says where each stops', () => {
		expect(LEVELS.map((level) => [level.id, level.label])).toEqual([
			['suggest', 'Read-only'],
			['stepByStep', 'Ask every step'],
			['signOff', 'Ask before pushing'],
			['unattended', 'Full access']
		]);
		expect(levelWords('suggest').says).toContain('Nothing runs');
		expect(levelWords('signOff').says).toContain('Push and Delete branch');
		expect(levelWords('unattended').says).toContain('still stops it');
	});

	it('says how many steps Run runs, or how many problems stand in the way', () => {
		expect(runLabel(6, 0)).toBe('Run · 6 steps');
		expect(runLabel(1, 0)).toBe('Run · 1 step');
		expect(runLabel(6, 1)).toBe('1 problem');
		expect(runLabel(6, 3)).toBe('3 problems');
	});

	it('picks out what was found about a step, and about the recipe', () => {
		const found = [error(null, 'recipe.unnamed', 'Name the recipe.'), warning(0, 'dirty', 'x'), error(2, 'a', 'y')];

		expect(findingsAt(found, null).map((finding) => finding.code)).toEqual(['recipe.unnamed']);
		expect(findingsAt(found, 0).map((finding) => finding.code)).toEqual(['dirty']);
		expect(findingsAt(found, 1)).toEqual([]);
	});

	it('writes how long a step took', () => {
		expect(took(null)).toBe('');
		expect(took(400)).toBe('0:00');
		expect(took(2000)).toBe('0:02');
		expect(took(80_000)).toBe('1:20');
	});

	it('says where a run is', () => {
		expect(runStatus(runOf(SYNC, 2), true)).toBe('Running · step 3 of 6');
		expect(runStatus(runOf(SYNC, 0), false)).toBe('Starting · step 1 of 6');
		expect(runStatus(conflicted().run, false)).toBe('Waiting for you · step 3 of 6');
		expect(runStatus(runOf(SYNC, 6, { outcome: 'done' }), false)).toBe('Done');
		expect(runStatus(runOf(SYNC, 2, { outcome: 'ended' }), false)).toBe('Ended at step 3 of 6');
		expect(runStatus(runOf(SYNC, 4, { outcome: 'undone' }), false)).toBe('Undone');
	});

	it('measures how much of a run is behind it', () => {
		expect(runShare(runOf(SYNC, 0))).toBe(0);
		expect(runShare(runOf(SYNC, 3))).toBe(0.5);
		expect(runShare(runOf(SYNC, 6))).toBe(1);
		expect(runShare(runOf(blank('empty'), 0))).toBe(0);
		// A step that was skipped is behind it too.
		const skipped = runOf(SYNC, 1);
		skipped.steps[1].state = 'skipped';
		expect(runShare(skipped)).toBeCloseTo(2 / 6);
	});

	it('writes each choice as its button says it', () => {
		const stop = conflicted().run.stop as Stop;

		expect(stop.choices.map((choice) => choiceLabel(choice, stop))).toEqual([
			'Open in Merger',
			'Open in Conflicts',
			'It is resolved: go on',
			'Abort the merge, keep steps 1–2',
			'End the run'
		]);
		expect(choiceLabel('continue', { ...stop, inMerger: true })).toBe('It is merged: go on');
		expect(choiceLabel('abort', { ...stop, step: 1 })).toBe('Abort the merge, keep step 1');
		expect(choiceLabel('abort', { ...stop, step: 0, operation: 'rebase' })).toBe('Abort the rebase');
		expect(choiceLabel('continue', { ...stop, kind: 'approve' })).toBe('Run step 3');
		expect(choiceLabel('continue', { ...stop, kind: 'ask' })).toBe('Go on');
		expect(choiceLabel('continue', { ...stop, kind: 'changed' })).toBe('Go on anyway');
		expect(choiceLabel('retry', stop)).toBe('Run it again');
		expect(choiceLabel('skip', stop)).toBe('Skip this step');
	});

	it('fills the button that goes on', () => {
		const stop = conflicted().run.stop as Stop;

		expect(mainChoice(stop)).toBe('openMerger');
		expect(mainChoice({ ...stop, choices: ['openConflicts', 'continue', 'end'] })).toBe('openConflicts');
		expect(mainChoice({ ...stop, inMerger: true, choices: ['continue', 'end'] })).toBe('continue');
		expect(mainChoice({ ...stop, kind: 'approve', choices: ['continue', 'end'] })).toBe('continue');
		expect(mainChoice({ ...stop, kind: 'failed', choices: ['retry', 'skip', 'end'] })).toBe('retry');
		expect(mainChoice({ ...stop, kind: 'failed', choices: ['end'] })).toBeNull();
	});
});
