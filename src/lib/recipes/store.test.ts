// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Recipes screen's state (FEAT-133): the draft and its live validation,
 * the agent's switch, and a run.
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	agentsSnapshot,
	checkOf,
	conflicted,
	error,
	runOf,
	settle,
	SQUASH,
	SYNC,
	viewOf
} from '../../testing/recipe-fixtures';
import type { Line, Recipe, RunView } from './types';

const { heard, confirm, notice, agentState } = vi.hoisted(() => ({
	heard: {} as Record<string, (event: { payload: unknown }) => void>,
	confirm: vi.fn(),
	notice: { ok: vi.fn(), failed: vi.fn() },
	agentState: { snapshot: null as unknown, load: vi.fn() }
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
		heard[name] = handler;
		return () => delete heard[name];
	})
}));
vi.mock('$lib/ui/dialog.svelte', () => ({ dialog: { confirm } }));
vi.mock('$lib/ui/notice.svelte', () => ({
	notice,
	describe: (cause: unknown) => (cause instanceof Error ? cause.message : String(cause))
}));
vi.mock('$lib/agents/store.svelte', () => ({
	agents: {
		get snapshot() {
			return agentState.snapshot;
		},
		load: agentState.load
	}
}));
vi.mock('$lib/agents/api', () => ({ consent: vi.fn(async () => {}) }));
vi.mock('./api', () => ({
	RUN_EVENT: 'recipe-run',
	LINE_EVENT: 'recipe-line',
	list: vi.fn(),
	save: vi.fn(),
	remove: vi.fn(),
	importFrom: vi.fn(),
	check: vi.fn(),
	propose: vi.fn(),
	start: vi.fn(),
	answer: vi.fn(),
	stop: vi.fn(async () => {}),
	undo: vi.fn(),
	current: vi.fn(),
	close: vi.fn(async () => {})
}));

import { activity } from '$lib/activity/store.svelte';
import * as agentsApi from '$lib/agents/api';
import * as api from './api';
import { recipes } from './store.svelte';

const list = vi.mocked(api.list);
const check = vi.mocked(api.check);
const save = vi.mocked(api.save);
const remove = vi.mocked(api.remove);
const importFrom = vi.mocked(api.importFrom);
const propose = vi.mocked(api.propose);
const start = vi.mocked(api.start);
const answer = vi.mocked(api.answer);
const undo = vi.mocked(api.undo);
const current = vi.mocked(api.current);
const close = vi.mocked(api.close);
const consent = vi.mocked(agentsApi.consent);

const blocks = () => recipes.draft?.steps.map((step) => step.block) ?? [];

beforeEach(() => {
	vi.clearAllMocks();
	recipes.clearState();
	activity.reset();
	for (const name of Object.keys(heard)) delete heard[name];
	agentState.snapshot = agentsSnapshot();
	list.mockResolvedValue({ recipes: [SYNC, SQUASH], guideVersion: 1 });
	current.mockResolvedValue(null);
	check.mockImplementation(async (recipe: Recipe) => checkOf(recipe));
	confirm.mockResolvedValue(true);
});

async function loaded(): Promise<void> {
	await recipes.load('/repos/thing');
	await settle(0);
}

describe('opening the screen', () => {
	it('reads the repository’s recipes and opens the first', async () => {
		await loaded();

		expect(recipes.saved.map((recipe) => recipe.name)).toEqual(['sync-and-ship', 'squash-for-pr']);
		expect(recipes.guideVersion).toBe(1);
		expect(recipes.draft).toEqual(SYNC);
		expect(recipes.savedAs).toBe('sync-and-ship');
		expect(recipes.dirty).toBe(false);
		expect(recipes.loaded).toBe(true);
		// Checked at once, not after the pause an edit waits for.
		expect(check).toHaveBeenCalledWith(SYNC);
		expect(recipes.check?.commands[0]).toBe('git stash push -u');
	});

	it('starts a new recipe in a repository that has none', async () => {
		list.mockResolvedValue({ recipes: [], guideVersion: 1 });

		await loaded();

		expect(recipes.draft?.name).toBe('recipe-1');
		expect(recipes.draft?.steps).toEqual([]);
		expect(recipes.savedAs).toBeNull();
		expect(recipes.draft?.agentHelp).toBe(false);
	});

	it('says so when the recipes cannot be read, and still gives a stack to build', async () => {
		list.mockRejectedValue('recipes.json could not be read, so it was left as it is');

		await loaded();

		expect(recipes.error).toBe('recipes.json could not be read, so it was left as it is');
		expect(recipes.draft?.name).toBe('recipe-1');
	});

	it('shows a run that is still going, and not one that is over', async () => {
		current.mockResolvedValue(conflicted());
		await loaded();
		expect(recipes.run?.run.stop?.kind).toBe('conflict');

		recipes.clearState();
		current.mockResolvedValue(viewOf(runOf(SYNC, 6, { outcome: 'done' })));
		await loaded();
		expect(recipes.run).toBeNull();

		// A record read back after Spagitty was closed is not a run to answer.
		recipes.clearState();
		current.mockResolvedValue(viewOf(runOf(SYNC, 2), { live: false }));
		await loaded();
		expect(recipes.run).toBeNull();
	});

	it('opens another saved recipe, and a new one with a name nothing has', async () => {
		await loaded();

		recipes.open('squash-for-pr');
		expect(recipes.draft).toEqual(SQUASH);
		expect(recipes.savedAs).toBe('squash-for-pr');

		recipes.create();
		expect(recipes.draft?.name).toBe('recipe-1');
		expect(recipes.savedAs).toBeNull();
		expect(recipes.dirty).toBe(true);

		recipes.open('nowhere');
		expect(recipes.draft?.name).toBe('recipe-1');
	});
});

describe('building a stack', () => {
	beforeEach(loaded);

	it('adds a block at the end with what can be chosen already chosen, and puts the keys on it', () => {
		recipes.add('fetch');
		recipes.add('check');

		expect(blocks().slice(-2)).toEqual(['fetch', 'check']);
		expect(recipes.draft?.steps.at(-1)).toEqual({ block: 'check', params: { command: 'cargo test' } });
		expect(recipes.selected).toBe(7);
		expect(recipes.dirty).toBe(true);
		// The saved recipe is as it was until Save.
		expect(recipes.saved[0].steps).toHaveLength(6);
	});

	it('removes, moves and changes a step', () => {
		recipes.remove(0);
		expect(blocks()).toEqual(['pull', 'merge', 'pop', 'commit', 'push']);

		recipes.select(1);
		recipes.move(1, -1);
		expect(blocks().slice(0, 2)).toEqual(['merge', 'pull']);
		// The keys go with the step that moved.
		expect(recipes.selected).toBe(0);

		recipes.set(0, { block: 'merge', params: { branch: 'old', how: 'squash' } });
		expect(recipes.draft?.steps[0]).toEqual({ block: 'merge', params: { branch: 'old', how: 'squash' } });

		// Past an end, nothing moves and nothing is remembered.
		recipes.move(0, -1);
		recipes.remove(99);
		expect(blocks().slice(0, 2)).toEqual(['merge', 'pull']);
	});

	it('keeps the selection inside the stack', () => {
		recipes.select(99);
		expect(recipes.selected).toBe(5);
		recipes.select(-7);
		expect(recipes.selected).toBe(-1);

		recipes.select(5);
		recipes.remove(5);
		expect(recipes.selected).toBe(4);
	});

	it('takes edits back one at a time', () => {
		expect(recipes.canUndo).toBe(false);
		recipes.remove(0);
		recipes.rename('renamed');
		expect(recipes.canUndo).toBe(true);

		recipes.undo();
		expect(recipes.draft?.name).toBe('sync-and-ship');
		expect(blocks()).toHaveLength(5);

		recipes.undo();
		expect(blocks()).toHaveLength(6);
		expect(recipes.dirty).toBe(false);
		recipes.undo();
		expect(blocks()).toHaveLength(6);
	});

	it('sets the level, the note, the name and the branch it is kept to', () => {
		recipes.setLevel('unattended');
		recipes.setNote('Be brief.');
		recipes.rename('ship');
		recipes.setBranch('feature/avatars');

		expect(recipes.draft).toMatchObject({
			level: 'unattended',
			note: 'Be brief.',
			name: 'ship',
			branch: 'feature/avatars'
		});
	});
});

describe('live validation', () => {
	beforeEach(loaded);

	/** The item: the validator runs on every change. */
	it('checks the stack after an edit, and shows what was found', async () => {
		check.mockClear();
		check.mockImplementation(async (recipe: Recipe) =>
			checkOf(recipe, [error(6, 'pop.noStash', 'There is no Stash above this to pop.')])
		);

		recipes.add('pop');
		expect(check).not.toHaveBeenCalled();
		await settle();

		expect(check).toHaveBeenCalledTimes(1);
		expect(check.mock.calls[0][0].steps).toHaveLength(7);
		expect(recipes.check?.errors).toBe(1);
		expect(recipes.check?.findings[0].text).toBe('There is no Stash above this to pop.');
	});

	it('checks once for a burst of edits, with the stack as it ended up', async () => {
		check.mockClear();

		recipes.rename('s');
		recipes.rename('sy');
		recipes.rename('syn');
		await settle();

		expect(check).toHaveBeenCalledTimes(1);
		expect(check.mock.calls[0][0].name).toBe('syn');
	});

	it('drops an answer about a stack that has changed since it was asked', async () => {
		let slow: (value: ReturnType<typeof checkOf>) => void = () => {};
		check.mockImplementationOnce(() => new Promise((resolve) => (slow = resolve)));
		recipes.rename('first');
		await settle();

		check.mockImplementation(async (recipe: Recipe) => checkOf(recipe));
		recipes.rename('second');
		await settle();
		slow(checkOf({ ...SYNC, name: 'first' }, [error(null, 'late', 'A late answer')]));
		await settle(0);

		expect(recipes.check?.findings).toEqual([]);
	});

	it('says so when the stack cannot be checked, and stops saying it when it can', async () => {
		check.mockRejectedValueOnce('No repository is open');
		recipes.rename('x');
		await settle();
		expect(recipes.error).toBe('No repository is open');

		recipes.rename('xy');
		await settle();
		expect(recipes.error).toBeNull();
	});
});

/** The author's note of 2026-10-13. */
describe('the agent’s help', () => {
	beforeEach(loaded);

	it('lists the agents that run and the models, and which need the repository’s agreement', () => {
		expect(recipes.helpers).toEqual([
			{ id: 'claude', name: 'Claude Code', detail: null, needsConsent: null },
			{
				id: 'sonnet',
				name: 'Sonnet',
				detail: 'claude-sonnet-5-5',
				needsConsent: { slug: 'anthropic', label: 'Anthropic' }
			},
			// On this machine: nothing leaves it, so nothing is asked.
			{ id: 'ollama', name: 'Local model', detail: 'qwen', needsConsent: null }
		]);

		agentState.snapshot = agentsSnapshot(['anthropic']);
		expect(recipes.helpers[1].needsConsent).toBeNull();

		agentState.snapshot = { ...agentsSnapshot(), rules: { ...agentsSnapshot().rules!, agents: ['ollama'] } };
		expect(recipes.helpers.map((helper) => helper.id)).toEqual(['ollama']);

		agentState.snapshot = null;
		expect(recipes.helpers).toEqual([]);
	});

	it('switched on, chooses the first agent that needs no asking', () => {
		recipes.setAgentHelp(true);

		expect(recipes.draft?.agentHelp).toBe(true);
		expect(recipes.draft?.agent).toBe('claude');
	});

	it('switched off, turns a message the agent would write into one to type', () => {
		recipes.open('squash-for-pr');

		recipes.setAgentHelp(false);

		expect(recipes.draft?.agentHelp).toBe(false);
		expect(recipes.draft?.steps[0]).toEqual({
			block: 'squash',
			params: { count: 3, message: { kind: 'text', text: '' } }
		});
		// Kept for when it is on again.
		expect(recipes.draft?.note).toBe('Keep it short.');
	});

	it('asks before choosing a model the repository has not agreed to send anything to', async () => {
		recipes.setAgentHelp(true);

		await recipes.setAgent('sonnet');

		expect(confirm).toHaveBeenCalledTimes(1);
		expect(confirm.mock.calls[0][0].title).toBe('Send to Anthropic?');
		expect(consent).toHaveBeenCalledWith('/repos/thing', 'anthropic');
		expect(agentState.load).toHaveBeenCalledWith('/repos/thing');
		expect(recipes.draft?.agent).toBe('sonnet');
	});

	it('does not choose it when the answer is no', async () => {
		recipes.setAgentHelp(true);
		confirm.mockResolvedValue(false);

		await recipes.setAgent('sonnet');

		expect(consent).not.toHaveBeenCalled();
		expect(recipes.draft?.agent).toBe('claude');
	});

	it('asks nothing for an agent on this machine', async () => {
		recipes.setAgentHelp(true);

		await recipes.setAgent('ollama');

		expect(confirm).not.toHaveBeenCalled();
		expect(recipes.draft?.agent).toBe('ollama');
	});

	it('puts the blocks the agent proposes in the draft, where they can be changed and taken back', async () => {
		recipes.setAgentHelp(true);
		recipes.setSentence('  Stash, pull and push  ');
		propose.mockResolvedValue({
			steps: [
				{ block: 'stash', params: { untracked: true } },
				{ block: 'pull', params: { remote: 'origin', mode: 'rebase' } },
				{ block: 'push', params: { remote: 'origin' } }
			],
			said: 'This stashes, pulls and pushes.'
		});

		await recipes.propose();

		expect(propose.mock.calls[0][1]).toBe('Stash, pull and push');
		expect(propose.mock.calls[0][0].agent).toBe('claude');
		expect(blocks()).toEqual(['stash', 'pull', 'push']);
		expect(recipes.said).toBe('This stashes, pulls and pushes.');
		expect(recipes.proposing).toBe(false);
		// Nothing ran.
		expect(start).not.toHaveBeenCalled();

		recipes.undo();
		expect(blocks()).toHaveLength(6);
	});

	it('keeps the stack when the agent proposes nothing, and says what it said', async () => {
		recipes.setAgentHelp(true);
		recipes.setSentence('Rewrite somebody else’s branch');
		propose.mockResolvedValue({ steps: [], said: 'These blocks cannot do that.' });

		await recipes.propose();

		expect(blocks()).toHaveLength(6);
		expect(recipes.said).toBe('These blocks cannot do that.');

		propose.mockResolvedValue({ steps: [], said: '' });
		await recipes.propose();
		expect(recipes.said).toBe('The agent proposed no blocks.');
	});

	it('says why when the agent cannot be asked', async () => {
		recipes.setAgentHelp(true);
		recipes.setSentence('Sync');
		propose.mockRejectedValue('Claude Code is not installed.');

		await recipes.propose();

		expect(notice.failed).toHaveBeenCalledWith('The agent could not propose blocks', 'Claude Code is not installed.');
		expect(blocks()).toHaveLength(6);
	});

	/** Off, nothing in the store asks an agent for anything. */
	it('asks no agent for anything while the help is off', async () => {
		recipes.setSentence('Sync and ship');

		await recipes.propose();

		expect(propose).not.toHaveBeenCalled();
		expect(recipes.draft?.agentHelp).toBe(false);
	});

	it('does not ask with nothing written, or with no agent chosen', async () => {
		recipes.setAgentHelp(true);
		recipes.setSentence('   ');
		await recipes.propose();

		recipes.setSentence('Sync');
		await recipes.setAgent(null);
		await recipes.propose();

		expect(propose).not.toHaveBeenCalled();
	});
});

describe('keeping a recipe', () => {
	beforeEach(loaded);

	it('saves the draft over the recipe it was opened from', async () => {
		recipes.setNote('changed');
		save.mockImplementation(async (recipe) => [recipe, SQUASH]);

		expect(await recipes.save()).toBe(true);

		expect(save).toHaveBeenCalledWith({ ...SYNC, note: 'changed' }, 'sync-and-ship');
		expect(recipes.dirty).toBe(false);
		expect(notice.ok).toHaveBeenCalledWith('Saved sync-and-ship');
	});

	it('saves a renamed recipe under its new name, in its old place', async () => {
		recipes.rename('  ship  ');
		save.mockImplementation(async (recipe) => [recipe, SQUASH]);

		await recipes.save();

		expect(save.mock.calls[0][0].name).toBe('ship');
		expect(save.mock.calls[0][1]).toBe('sync-and-ship');
		expect(recipes.savedAs).toBe('ship');
		expect(recipes.draft?.name).toBe('ship');
	});

	it('keeps the draft when it cannot be saved', async () => {
		recipes.rename('squash-for-pr');
		save.mockRejectedValue('There is already a recipe called squash-for-pr.');

		expect(await recipes.save()).toBe(false);

		expect(notice.failed).toHaveBeenCalledWith(
			'Could not save the recipe',
			'There is already a recipe called squash-for-pr.'
		);
		expect(recipes.savedAs).toBe('sync-and-ship');
		expect(recipes.dirty).toBe(true);
	});

	it('deletes after asking, and opens what is left', async () => {
		remove.mockResolvedValue([SQUASH]);

		await recipes.removeSaved();

		expect(confirm.mock.calls[0][0].title).toBe('Delete sync-and-ship?');
		expect(remove).toHaveBeenCalledWith('sync-and-ship');
		expect(recipes.savedAs).toBe('squash-for-pr');

		confirm.mockResolvedValue(false);
		await recipes.removeSaved();
		expect(remove).toHaveBeenCalledTimes(1);
	});

	it('exports the draft as JSON and opens one that is imported, unsaved', async () => {
		expect(JSON.parse(recipes.exported())).toEqual(SYNC);

		importFrom.mockResolvedValue({ ...SQUASH, name: 'from-a-colleague' });
		expect(await recipes.importFrom('{…}')).toBe(true);
		expect(recipes.draft?.name).toBe('from-a-colleague');
		expect(recipes.savedAs).toBeNull();

		// A name that is taken is not written over.
		importFrom.mockResolvedValue(SYNC);
		await recipes.importFrom('{…}');
		expect(recipes.draft?.name).toBe('recipe-1');

		importFrom.mockRejectedValue('That is not a recipe: expected value');
		expect(await recipes.importFrom('nonsense')).toBe(false);
		expect(notice.failed).toHaveBeenCalledWith('Could not read that recipe', 'That is not a recipe: expected value');
	});
});

describe('a run', () => {
	beforeEach(loaded);

	const send = (view: RunView) => heard['recipe-run']?.({ payload: view });
	const print = (line: Line, id = '1800000000-ab12') =>
		heard['recipe-line']?.({ payload: { repo: '/repos/thing', id, line } });

	it('starts with the draft as it is, and shows the run', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 0)));

		expect(await recipes.start()).toBe(true);

		expect(start).toHaveBeenCalledWith(SYNC);
		expect(recipes.run?.run.id).toBe('1800000000-ab12');
		expect(recipes.lines).toHaveLength(1);
		expect(recipes.starting).toBe(false);
	});

	it('says why a recipe did not start, and checks it again', async () => {
		start.mockRejectedValue('There is no branch main, and no step above makes it.');
		check.mockClear();

		expect(await recipes.start()).toBe(false);
		await settle(0);

		expect(notice.failed).toHaveBeenCalledWith(
			'The recipe did not start',
			'There is no branch main, and no step above makes it.'
		);
		expect(recipes.run).toBeNull();
		expect(check).toHaveBeenCalled();
	});

	it('follows the run as it is sent, and the lines printed in between', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 0)));
		await recipes.start();

		print({ step: 0, kind: 'command', text: '$ git stash push -u' });
		print({ step: 0, kind: 'out', text: 'Saved working directory' });
		expect(recipes.lines.map((line) => line.text)).toEqual([
			'restore point refs/spagitty/restore/1800000000-ab12 saved',
			'$ git stash push -u',
			'Saved working directory'
		]);

		// The whole run takes their place: no line is shown twice.
		const next = runOf(SYNC, 1);
		next.lines.push({ step: 0, kind: 'command', text: '$ git stash push -u' });
		send(viewOf(next, { working: true }));
		expect(recipes.lines).toHaveLength(2);
		expect(recipes.run?.working).toBe(true);
		expect(recipes.run?.run.at).toBe(1);
	});

	/** FEAT-121: what takes time says so, and what would collide waits. */
	it('holds the activity lanes while a step runs, and lets them go while it waits', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 0), { working: true }));
		await recipes.start();
		expect(activity.label).toBe('Running sync-and-ship · step 1 of 6…');
		expect(activity.blocked('repository')).toBe('Running sync-and-ship · step 1 of 6 is still running');
		expect(activity.blocked('network')).not.toBeNull();

		send(viewOf(runOf(SYNC, 1), { working: true }));
		expect(activity.label).toBe('Running sync-and-ship · step 2 of 6…');

		// Waiting for the person: resolving a conflict is theirs to do.
		send(conflicted());
		expect(activity.running).toBe(false);
		expect(activity.blocked('repository', 'network')).toBeNull();

		send(viewOf(runOf(SYNC, 3), { working: true }));
		expect(activity.running).toBe(true);
		send(viewOf(runOf(SYNC, 6, { outcome: 'done' })));
		expect(activity.running).toBe(false);
	});

	it('does not start while something else is writing to the repository', async () => {
		const pull = activity.begin('pull', 'Pulling', ['repository', 'network']);

		expect(await recipes.start()).toBe(false);

		expect(start).not.toHaveBeenCalled();
		expect(notice.failed).toHaveBeenCalledWith('The recipe did not start', 'Pulling is still running');
		pull?.end();
	});

	it('takes no line and no run that is another repository’s or another run’s', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 0)));
		await recipes.start();

		print({ step: 0, kind: 'out', text: 'someone else’s' }, 'another-run');
		send({ ...viewOf(runOf(SYNC, 3)), repo: '/repos/elsewhere' });
		send(viewOf(runOf(SYNC, 4, { id: 'another-run' })));

		expect(recipes.lines).toHaveLength(1);
		expect(recipes.run?.run.at).toBe(0);
	});

	it('takes nothing with no run on screen', () => {
		send(viewOf(runOf(SYNC, 3)));

		expect(recipes.run).toBeNull();
	});

	it('gives the person’s choice, and keeps waiting when it cannot be taken yet', async () => {
		start.mockResolvedValue(conflicted());
		await recipes.start();
		answer.mockRejectedValueOnce('1 file is still conflicted.');

		expect(await recipes.answer('continue')).toBe(false);
		expect(recipes.refused).toBe('1 file is still conflicted.');
		expect(recipes.run?.run.stop?.kind).toBe('conflict');

		answer.mockResolvedValue(viewOf(runOf(SYNC, 3)));
		expect(await recipes.answer('continue')).toBe(true);
		expect(answer).toHaveBeenLastCalledWith('continue');
		expect(recipes.refused).toBeNull();
		expect(recipes.run?.run.stop).toBeNull();
		expect(recipes.answering).toBe(false);
	});

	it('answers nothing with no run', async () => {
		expect(await recipes.answer('continue')).toBe(false);
		expect(answer).not.toHaveBeenCalled();
	});

	it('stops a run', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 1)));
		await recipes.start();

		await recipes.stop();

		expect(api.stop).toHaveBeenCalledTimes(1);
	});

	it('undoes the whole run after asking, and not when the answer is no', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 4)));
		await recipes.start();
		confirm.mockResolvedValueOnce(false);

		await recipes.undoRun();
		expect(undo).not.toHaveBeenCalled();

		undo.mockResolvedValue(viewOf(runOf(SYNC, 4, { outcome: 'undone' })));
		await recipes.undoRun();
		expect(confirm.mock.calls[1][0].body).toContain('What was pushed stays pushed.');
		expect(recipes.run?.run.outcome).toBe('undone');
		expect(notice.ok).toHaveBeenCalledWith('The run was undone');
	});

	it('says why a run could not be undone', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 4)));
		await recipes.start();
		undo.mockRejectedValue('A step is running. Stop it first.');

		await recipes.undoRun();

		expect(notice.failed).toHaveBeenCalledWith('Could not undo the run', 'A step is running. Stop it first.');
	});

	it('leaves a run that is over, back to the builder', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 6, { outcome: 'done' })));
		await recipes.start();

		await recipes.closeRun();

		expect(close).toHaveBeenCalledTimes(1);
		expect(recipes.run).toBeNull();
		expect(recipes.draft).toEqual(SYNC);
	});

	it('stays on a run that is not over', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 2)));
		await recipes.start();
		close.mockRejectedValueOnce('The run is not over. Stop it first.');

		await recipes.closeRun();

		expect(recipes.run).not.toBeNull();
		expect(notice.failed).toHaveBeenCalledWith('The run is not over', 'The run is not over. Stop it first.');
	});
});
