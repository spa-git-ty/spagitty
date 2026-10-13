// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Recipes, mounted (FEAT-133): the builder's keys and live validation, the
 * agent's switch, and a run.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import {
	agentsSnapshot,
	checkOf,
	conflicted,
	error,
	runOf,
	settle as wait,
	SQUASH,
	SYNC,
	viewOf,
	warning
} from '../../testing/recipe-fixtures';
import type { Recipe, RunView } from '$lib/recipes/types';
import type { RepoInfo } from '$lib/types';

const { goto, heard, confirm, notice, agentState, merger } = vi.hoisted(() => ({
	goto: vi.fn(),
	heard: {} as Record<string, (event: { payload: unknown }) => void>,
	confirm: vi.fn(),
	notice: { ok: vi.fn(), failed: vi.fn() },
	agentState: { snapshot: null as unknown, load: vi.fn() },
	merger: { pickA: vi.fn(), pickB: vi.fn() }
}));

vi.mock('$app/navigation', () => ({ goto }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true)
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
vi.mock('$lib/merger/store.svelte', () => ({ merger }));
vi.mock('$lib/recipes/api', () => ({
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
import * as api from '$lib/recipes/api';
import { recipes } from '$lib/recipes/store.svelte';
import Recipes from './+page.svelte';

const list = vi.mocked(api.list);
const check = vi.mocked(api.check);
const save = vi.mocked(api.save);
const importFrom = vi.mocked(api.importFrom);
const propose = vi.mocked(api.propose);
const start = vi.mocked(api.start);
const answer = vi.mocked(api.answer);
const undo = vi.mocked(api.undo);
const current = vi.mocked(api.current);

function repository(): RepoInfo {
	return {
		path: '/repos/thing',
		name: 'thing',
		bare: false,
		lastFetched: null,
		head: { branch: 'feature/avatars', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
	};
}

let view: Mounted | null = null;
const writeText = vi.fn((_text: string) => Promise.resolve());

async function settle(ms = 160): Promise<void> {
	await wait(ms);
	flushSync();
}

async function show(): Promise<Mounted> {
	view = render(Recipes, {});
	await settle(0);
	return view;
}

const said = (selector: string) => view!.get(selector).textContent?.replace(/\s+/g, ' ').trim() ?? '';
const button = (name: string) => view!.all('button').find((b) => b.textContent?.trim() === name) as HTMLButtonElement;
const cards = () => view!.all('.stack .card');
const stack = () => cards().map((card) => card.querySelector('.name')?.textContent?.trim());
const commands = () => view!.all('.stack .command').map((each) => each.textContent?.trim());
const labelled = <T extends HTMLElement = HTMLElement>(label: string) =>
	view!.find(`[aria-label="${label}"]`) as T | null;
const key = (key: string, init: KeyboardEventInit = {}) => press(window, key, init);

function choose(select: HTMLSelectElement, value: string) {
	select.value = value;
	fire(select, 'change');
}

function type(field: HTMLInputElement | HTMLTextAreaElement, text: string) {
	field.value = text;
	fire(field, 'input');
}

beforeEach(() => {
	vi.clearAllMocks();
	recipes.clearState();
	activity.reset();
	control.reset();
	control.setInfo(repository());
	for (const name of Object.keys(heard)) delete heard[name];
	agentState.snapshot = agentsSnapshot();
	list.mockResolvedValue({ recipes: [SYNC, SQUASH], guideVersion: 1 });
	current.mockResolvedValue(null);
	check.mockImplementation(async (recipe: Recipe) => checkOf(recipe));
	confirm.mockResolvedValue(true);
	vi.stubGlobal('navigator', { ...navigator, clipboard: { writeText } });
});

afterEach(() => {
	view?.destroy();
	view = null;
	vi.unstubAllGlobals();
});

describe('the builder', () => {
	it('shows the saved recipes, and the one opened as its blocks with their commands', async () => {
		await show();

		expect(view!.all('.chips [role="tab"]').map((tab) => [tab.textContent?.trim(), tab.getAttribute('aria-selected')])).toEqual([
			['sync-and-ship', 'true'],
			['squash-for-pr', 'false']
		]);
		expect(stack()).toEqual(['Stash', 'Pull', 'Merge', 'Pop stash', 'Commit', 'Push']);
		expect(commands()).toEqual([
			'git stash push -u',
			'git pull origin',
			'git merge main',
			'git stash pop',
			'git commit -m "Sync with main"',
			'git push --set-upstream origin HEAD'
		]);
		expect(labelled<HTMLInputElement>("The recipe's name")?.value).toBe('sync-and-ship');
		expect(said('.start')).toContain('On Run');
		expect(button('Run · 6 steps').disabled).toBe(false);
	});

	it('lists the eighteen blocks by group, each with its key', async () => {
		await show();

		expect(view!.all('.catalogue section').map((group) => group.getAttribute('aria-label'))).toEqual([
			'Save',
			'Sync',
			'Combine',
			'Branch',
			'Shape history',
			'Guard'
		]);
		const blocks = view!.all('.catalogue .block');
		expect(blocks).toHaveLength(18);
		expect(blocks[0].textContent?.replace(/\s+/g, ' ').trim()).toBe('Stash S');
		expect(blocks[17].textContent?.replace(/\s+/g, ' ').trim()).toBe('Stop unless clean G');
	});

	it('adds a block when it is pressed in the catalogue', async () => {
		await show();

		click(view!.all('.catalogue .block').find((block) => block.textContent?.includes('Fetch'))!);

		expect(stack().at(-1)).toBe('Fetch');
		expect(cards().at(-1)?.closest('.row')?.classList.contains('selected')).toBe(true);
	});

	it('opens another recipe from its chip, and a new one from +', async () => {
		await show();

		click(view!.all('.chips [role="tab"]')[1]);
		expect(stack()).toEqual(['Squash']);

		click(labelled('New recipe')!);
		expect(stack()).toEqual([]);
		expect(said('.stack')).toContain('No blocks yet. Press a block on the left, or its key.');
		expect(labelled<HTMLInputElement>("The recipe's name")?.value).toBe('recipe-1');
	});

	it('says there is no repository when none is open', async () => {
		control.setInfo(null);

		await show();

		expect(view!.text()).toContain('Open a repository to build a recipe for it.');
		expect(view!.find('.catalogue')).toBeNull();
		expect(list).not.toHaveBeenCalled();
	});
});

/** The item: component tests for the builder's keys. */
describe('the builder’s keys', () => {
	it('adds a block with its letter', async () => {
		await show();

		key('f');
		key('G', { shiftKey: true });

		expect(stack().slice(-2)).toEqual(['Fetch', 'Stop unless clean']);
	});

	it('walks the stack with j and k, and reorders with Alt and the arrows', async () => {
		await show();
		const selected = () => cards().findIndex((card) => card.closest('.row')?.classList.contains('selected'));

		key('j');
		key('j');
		expect(selected()).toBe(1);
		key('k');
		expect(selected()).toBe(0);

		key('ArrowDown', { altKey: true });
		expect(stack().slice(0, 2)).toEqual(['Pull', 'Stash']);
		expect(selected()).toBe(1);
		key('ArrowUp', { altKey: true });
		expect(stack().slice(0, 2)).toEqual(['Stash', 'Pull']);
	});

	it('removes the selected block with Delete, and takes it back with Ctrl+Z', async () => {
		await show();

		key('j');
		key('Delete');
		expect(stack()).toEqual(['Pull', 'Merge', 'Pop stash', 'Commit', 'Push']);

		key('z', { ctrlKey: true });
		expect(stack()).toHaveLength(6);

		// With nothing selected there is nothing to remove.
		click(labelled('New recipe')!);
		key('s');
		recipes.select(-1);
		key('Delete');
		expect(stack()).toEqual(['Stash']);
	});

	it('runs with Ctrl+Enter, and not a stack that cannot run', async () => {
		start.mockResolvedValue(viewOf(runOf(SYNC, 0)));
		await show();

		key('Enter', { ctrlKey: true });
		await settle(0);
		expect(start).toHaveBeenCalledTimes(1);

		view!.destroy();
		recipes.clearState();
		start.mockClear();
		check.mockImplementation(async (recipe: Recipe) => checkOf(recipe, [error(0, 'x', 'No.')]));
		await show();
		key('Enter', { ctrlKey: true });
		await settle(0);
		expect(start).not.toHaveBeenCalled();
	});

	/** A letter typed into a commit message is a letter. */
	it('leaves the keys alone while a field has the focus', async () => {
		await show();
		const message = labelled<HTMLInputElement>('Step 5 message')!;

		press(message, 's');
		press(message, 'Delete');
		press(labelled("The recipe's name")!, 'p');

		expect(stack()).toHaveLength(6);
	});

	it('lists its keys under the stack', async () => {
		await show();

		expect(view!.all('.hints > span').map((hint) => hint.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'S P C … add a block',
			'J K move',
			'Alt ↑↓ reorder',
			'Del remove',
			'Ctrl ↵ run',
			'Ctrl Z undo'
		]);
	});
});

/** The item: component tests for live validation. */
describe('live validation', () => {
	it('shows what the validator found under the block it is about, and stops Run', async () => {
		check.mockImplementation(async (recipe: Recipe) =>
			checkOf(recipe, [
				error(2, 'branch.missing', 'There is no branch main, and no step above makes it.'),
				warning(0, 'stash.kept', 'No Pop follows: the changes stay stashed.')
			])
		);

		await show();

		const findings = view!.all('.stack .finding');
		expect(findings.map((each) => each.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'· No Pop follows: the changes stay stashed.',
			'! There is no branch main, and no step above makes it.'
		]);
		expect(findings[1].getAttribute('role')).toBe('alert');
		expect(findings[0].classList.contains('error')).toBe(false);
		// Under its own block: the warning follows the first card, the error the third.
		expect(findings[0].previousElementSibling).toBe(cards()[0].closest('.row'));
		expect(findings[1].previousElementSibling).toBe(cards()[2].closest('.row'));
		const run = button('1 problem');
		expect(run.disabled).toBe(true);
	});

	it('checks again after an edit, and lets the stack run once nothing is wrong', async () => {
		check.mockImplementation(async (recipe: Recipe) =>
			checkOf(recipe, recipe.steps.length === 6 ? [error(3, 'pop.noStash', 'There is no Stash above this to pop.')] : [])
		);
		await show();
		expect(button('1 problem').disabled).toBe(true);

		click(labelled('Remove step 4')!);
		await settle();

		expect(view!.all('.stack .finding')).toEqual([]);
		expect(button('Run · 5 steps').disabled).toBe(false);
	});

	it('shows what is wrong with the recipe itself under On Run', async () => {
		check.mockImplementation(async (recipe: Recipe) =>
			checkOf(recipe, [error(null, 'repo.busy', 'A merge is in progress. Finish or abort it first.')])
		);

		await show();

		expect(view!.get('.start').nextElementSibling?.textContent?.trim()).toBe(
			'A merge is in progress. Finish or abort it first.'
		);
	});

	it('says when the stack could not be checked', async () => {
		check.mockRejectedValue('No repository is open');

		await show();

		expect(said('.error')).toBe('No repository is open');
	});
});

describe('a block’s parameters', () => {
	it('are chosen from lists', async () => {
		await show();

		choose(labelled<HTMLSelectElement>('Step 1: what is stashed')!, 'tracked');
		choose(labelled<HTMLSelectElement>('Step 2: how')!, 'rebase');
		choose(labelled<HTMLSelectElement>('Step 3: how')!, 'squash');
		choose(labelled<HTMLSelectElement>('Step 5: what is committed')!, 'staged');
		choose(labelled<HTMLSelectElement>('Step 6: remote')!, 'upstream');

		expect(recipes.draft?.steps).toEqual([
			{ block: 'stash', params: { untracked: false } },
			{ block: 'pull', params: { remote: 'origin', mode: 'rebase' } },
			{ block: 'merge', params: { branch: 'main', how: 'squash' } },
			{ block: 'pop', params: {} },
			{ block: 'commit', params: { all: false, message: { kind: 'text', text: 'Sync with main' } } },
			{ block: 'push', params: { remote: 'upstream' } }
		]);
		// The remotes offered are the repository's.
		expect([...labelled<HTMLSelectElement>('Step 6: remote')!.options].map((option) => option.value)).toEqual([
			'origin',
			'upstream'
		]);
		// And there is no way to force a push.
		expect(cards()[5].textContent).toContain('no force');
	});

	it('takes a message as it is typed, and counts it', async () => {
		await show();
		const message = labelled<HTMLInputElement>('Step 5 message')!;

		type(message, 'Bring main in');

		expect(recipes.draft?.steps[4]).toEqual({
			block: 'commit',
			params: { all: true, message: { kind: 'text', text: 'Bring main in' } }
		});
		expect(cards()[4].querySelector('.count')?.textContent).toBe('13/72');

		type(message, 'm'.repeat(80));
		expect(cards()[4].querySelector('.count')?.classList.contains('over')).toBe(true);
	});

	it('chooses a branch from the branch menu, with the branches the repository has', async () => {
		await show();

		click(labelled('Step 3: branch to merge: main')!);
		const rows = [...document.querySelectorAll<HTMLElement>('[role="option"]')];
		expect(rows.map((row) => row.title)).toEqual(['feature/avatars', 'main', 'old']);
		// The branch the step holds is the one ticked.
		expect(rows.map((row) => row.getAttribute('aria-selected'))).toEqual(['false', 'true', 'false']);

		click(rows[2]);

		expect(recipes.draft?.steps[2]).toEqual({ block: 'merge', params: { branch: 'old', how: 'commit' } });
		expect(document.querySelector('[role="option"]')).toBeNull();
	});

	it('offers a branch a step above makes', async () => {
		list.mockResolvedValue({
			recipes: [
				{
					...SYNC,
					steps: [
						{ block: 'newBranch', params: { name: 'topic', from: '' } },
						{ block: 'switch', params: { branch: '' } }
					]
				}
			],
			guideVersion: 1
		});
		await show();

		click(labelled('Step 2: branch to switch to: none chosen')!);

		const names = [...document.querySelectorAll('[role="option"]')].map((row) => row.textContent?.trim());
		expect(names.some((name) => name?.includes('topic'))).toBe(true);
	});

	it('moves a block up and removes it from its own buttons', async () => {
		await show();

		expect(labelled<HTMLButtonElement>('Move step 1 up')?.disabled).toBe(true);
		click(labelled('Move step 2 up')!);
		expect(stack().slice(0, 2)).toEqual(['Pull', 'Stash']);

		click(labelled('Remove step 1')!);
		expect(stack()[0]).toBe('Stash');
	});

	it('picks one of the repository’s checks, and says when it has none', async () => {
		await show();

		key('t');
		const picked = labelled<HTMLSelectElement>('Step 7: check')!;
		expect([...picked.options].map((option) => option.value)).toEqual(['cargo test', 'bun test']);
		choose(picked, 'bun test');
		expect(recipes.draft?.steps[6]).toEqual({ block: 'check', params: { command: 'bun test' } });

		view!.destroy();
		recipes.clearState();
		check.mockImplementation(async (recipe: Recipe) =>
			checkOf(recipe, [], { ...checkOf(recipe).world, checks: [] })
		);
		await show();
		key('t');
		expect(cards()[6].textContent).toContain('This repository has no checks.');
	});
});

/** The author's note of 2026-10-13. */
describe('the agent’s help', () => {
	it('is off for a recipe that never had it: its text is disabled and nobody is called', async () => {
		await show();

		expect(said('.panel .row')).toBe('Agent help off');
		expect(said('.panel')).toContain('Off: no agent is called. The run is the git commands below and nothing else.');
		expect(labelled('Agent')).toBeNull();
		const [sentence, note] = view!.all('.panel textarea') as HTMLTextAreaElement[];
		expect(sentence.disabled).toBe(true);
		expect(note.disabled).toBe(true);
		expect(button('Propose blocks').disabled).toBe(true);
		// *Agent writes* is there, and cannot be chosen.
		const writes = view!.all('.stack .toggle') as HTMLButtonElement[];
		expect(writes).toHaveLength(1);
		expect(writes[0].disabled).toBe(true);
		expect(writes[0].title).toBe('Agent help is off');
		// What is shown is the commands, as a script. There is no brief.
		expect(labelled('The commands, as a script')?.textContent).toContain('git stash push -u');
		expect(labelled('The brief, as it is sent')).toBeNull();
		expect(view!.find('.guide')).toBeNull();
		// A commit message is still typed.
		expect(labelled<HTMLInputElement>('Step 5 message')?.disabled).toBe(false);
	});

	it('switched on, offers the agents, the sentence, the note, and the brief with its guide’s version', async () => {
		await show();

		click(view!.get('.panel .row button'));
		await settle();

		expect(said('.panel .row')).toBe('Agent help on');
		expect([...labelled<HTMLSelectElement>('Agent')!.options].map((option) => option.textContent?.trim())).toEqual([
			'Choose an agent',
			'Claude Code',
			'Sonnet · claude-sonnet-5-5',
			'Local model · qwen'
		]);
		expect(labelled<HTMLSelectElement>('Agent')!.value).toBe('claude');
		const [sentence, note] = view!.all('.panel textarea') as HTMLTextAreaElement[];
		expect(sentence.disabled).toBe(false);
		expect(note.disabled).toBe(false);
		expect(labelled('The brief, as it is sent')?.textContent).toContain('recipe: sync-and-ship');
		expect(said('.guide')).toBe('recipes.md v1');
		expect((view!.get('.stack .toggle') as HTMLButtonElement).disabled).toBe(false);
	});

	it('proposes the blocks from a sentence, into the stack, and runs nothing', async () => {
		propose.mockResolvedValue({
			steps: [
				{ block: 'fetch', params: { remote: '' } },
				{ block: 'rebase', params: { onto: 'main' } }
			],
			said: 'This fetches and rebases onto main.'
		});
		await show();
		click(view!.get('.panel .row button'));
		const sentence = view!.all('.panel textarea')[0] as HTMLTextAreaElement;

		expect(button('Propose blocks').disabled).toBe(true);
		type(sentence, 'Get up to date with main');
		expect(view!.all('.panel .count')[0].textContent).toBe('24/280');
		click(button('Propose blocks'));
		await settle();

		expect(propose.mock.calls[0][1]).toBe('Get up to date with main');
		expect(stack()).toEqual(['Fetch', 'Rebase']);
		expect(said('.panel .said')).toBe('This fetches and rebases onto main.');
		expect(start).not.toHaveBeenCalled();
		// Read and edited like any other stack: it can be taken back.
		key('z', { ctrlKey: true });
		expect(stack()).toHaveLength(6);
	});

	it('leaves a message to the agent, and takes it back', async () => {
		await show();
		click(view!.get('.panel .row button'));
		const writes = view!.get('.stack .toggle');

		click(writes);
		expect(recipes.draft?.steps[4]).toEqual({ block: 'commit', params: { all: true, message: { kind: 'agent' } } });
		expect(cards()[4].textContent).toContain('the agent writes it when the step runs');
		expect(labelled('Step 5 message')).toBeNull();

		click(view!.get('.stack .toggle'));
		expect(labelled<HTMLInputElement>('Step 5 message')?.value).toBe('');
	});

	it('switched off again, gives back the message to type', async () => {
		await show();
		click(view!.all('.chips [role="tab"]')[1]);
		expect(cards()[0].textContent).toContain('the agent writes it when the step runs');

		click(view!.get('.panel .row button'));

		expect(labelled<HTMLInputElement>('Step 1 message')?.value).toBe('');
		expect((view!.get('.stack .toggle') as HTMLButtonElement).disabled).toBe(true);
	});

	it('keeps showing the agent in use when a model is refused', async () => {
		confirm.mockResolvedValue(false);
		await show();
		click(view!.get('.panel .row button'));
		const agent = labelled<HTMLSelectElement>('Agent')!;
		expect(agent.value).toBe('claude');

		choose(agent, 'sonnet');
		await settle(0);

		expect(confirm.mock.calls[0][0].title).toBe('Send to Anthropic?');
		expect(recipes.draft?.agent).toBe('claude');
		// The select says what is true, not what was refused.
		expect(labelled<HTMLSelectElement>('Agent')!.value).toBe('claude');
	});

	it('says there is no agent to help when none is set up', async () => {
		agentState.snapshot = { ...agentsSnapshot(), local: [], remote: [] };
		await show();

		click(view!.get('.panel .row button'));

		expect(said('.panel')).toContain('No agent is set up. Add one in Settings › Agents.');
		expect(button('Propose blocks').disabled).toBe(true);
	});
});

describe('the level', () => {
	it('is one of four, and says where it stops', async () => {
		await show();
		const levels = () => view!.all('.levels [role="radio"]');

		expect(levels().map((level) => [level.textContent?.trim(), level.getAttribute('aria-checked')])).toEqual([
			['Read-only', 'false'],
			['Every step', 'true'],
			['Before pushing', 'false'],
			['Full access', 'false']
		]);
		expect(said('.panel')).toContain('Stops before every step.');

		click(levels()[2]);
		await settle();
		expect(recipes.draft?.level).toBe('signOff');
		expect(said('.panel')).toContain('Runs, and stops before Push and Delete branch.');
	});

	it('at Read-only runs nothing, and says the script is the way out', async () => {
		await show();

		click(view!.all('.levels [role="radio"]')[0]);
		await settle();

		const run = button('Run · 6 steps');
		expect(run.disabled).toBe(true);
		expect(run.title).toBe('Read-only runs nothing. Copy the script, or choose another level.');
		expect(said('.panel')).toContain('Copy as script is the way out.');
	});

	it('shows the level the repository allows when the recipe asks for more', async () => {
		check.mockImplementation(async (recipe: Recipe) =>
			checkOf(recipe, [], { ...checkOf(recipe).world, highest: 'stepByStep' })
		);
		await show();

		click(view!.all('.levels [role="radio"]')[3]);
		await settle();

		expect(said('.panel')).toContain('Stops before every step.');
	});
});

describe('keeping and sharing', () => {
	it('saves only a recipe that has changed', async () => {
		save.mockImplementation(async (recipe) => [recipe, SQUASH]);
		await show();
		expect(button('Save').disabled).toBe(true);

		type(labelled<HTMLInputElement>("The recipe's name")!, 'ship-it');
		expect(button('Save').disabled).toBe(false);
		click(button('Save'));
		await settle(0);

		expect(save).toHaveBeenCalledWith({ ...SYNC, name: 'ship-it' }, 'sync-and-ship');
		expect(button('Save').disabled).toBe(true);
	});

	it('copies the steps as a script, and the recipe as JSON', async () => {
		await show();

		click(button('Copy as script'));
		await settle(0);
		expect(writeText).toHaveBeenLastCalledWith(expect.stringContaining('#!/bin/sh\nset -e\ngit stash push -u'));
		expect(notice.ok).toHaveBeenCalledWith('Script copied');

		click(button('Export'));
		await settle(0);
		expect(JSON.parse(writeText.mock.calls.at(-1)![0] as string)).toEqual(SYNC);
	});

	it('opens a recipe from the JSON pasted in', async () => {
		importFrom.mockResolvedValue({ ...SQUASH, name: 'from-a-colleague' });
		await show();

		click(button('Import'));
		const pasted = labelled<HTMLTextAreaElement>("A recipe's exported JSON")!;
		expect(button('Open it').disabled).toBe(true);
		type(pasted, '{"v":1}');
		click(button('Open it'));
		await settle(0);

		expect(importFrom).toHaveBeenCalledWith('{"v":1}');
		expect(labelled<HTMLInputElement>("The recipe's name")?.value).toBe('from-a-colleague');
		expect(labelled("A recipe's exported JSON")).toBeNull();
	});

	it('keeps a recipe to one branch, or lets it run on any', async () => {
		await show();
		const kept = () => view!.get('.head .branch');
		expect(kept().textContent?.trim()).toBe('feature/avatars · any branch');

		click(kept());
		expect(recipes.draft?.branch).toBe('feature/avatars');
		expect(kept().textContent?.trim()).toBe('feature/avatars');

		click(kept());
		expect(recipes.draft?.branch).toBe('');
	});
});

describe('a run', () => {
	async function running(first: RunView): Promise<void> {
		current.mockResolvedValue(first);
		await show();
	}

	it('takes the place of the builder, with each step’s state and time', async () => {
		await running(conflicted());

		expect(view!.find('.catalogue')).toBeNull();
		expect(said('.head .name')).toBe('sync-and-ship');
		expect(said('.head .status')).toBe('Waiting for you · step 3 of 6');
		expect(view!.all('.steps li').map((step) => [step.className.split(' ')[0], step.querySelector('.label')?.textContent, step.querySelector('.time')?.textContent])).toEqual([
			['done', 'Stash', '0:02'],
			['done', 'Pull', '0:03'],
			['waiting', 'Merge', '0:09'],
			['notStarted', 'Pop stash', ''],
			['notStarted', 'Commit', ''],
			['notStarted', 'Push', '']
		]);
		expect(view!.get('.meter').getAttribute('aria-valuenow')).toBe('2');
	});

	it('shows the commands Spagitty ran and what they printed, each in its own colour', async () => {
		await running(conflicted());

		expect(view!.all('.output .line').map((line) => [line.className.replace('line ', '').split(' ')[0], line.textContent])).toEqual([
			['note', 'restore point refs/spagitty/restore/1800000000-ab12 saved'],
			['command', '$ git stash push -u'],
			['out', 'Saved working directory and index state WIP on feature/avatars'],
			['command', '$ git merge --no-ff --no-edit main'],
			['error', 'CONFLICT (content): Merge conflict in crates/spagitty-core/src/avatars.rs'],
			['note', 'Merge stopped on 1 conflict']
		]);
		// With the help off, no agent is named.
		expect(said('.who')).toContain('Git commands only');
		expect(said('.who')).toContain('Spagitty runs every command · on feature/avatars');
	});

	it('shows the stop as one card with its choices', async () => {
		await running(conflicted());

		expect(said('.stop .title')).toBe('Merge stopped on 1 conflict crates/spagitty-core/src/avatars.rs');
		expect(view!.all('.stop .choices button').map((choice) => choice.textContent?.trim())).toEqual([
			'Open in Merger',
			'Open in Conflicts',
			'It is resolved: go on',
			'Abort the merge, keep steps 1–2',
			'End the run'
		]);
		expect(view!.get('.stop').getAttribute('role')).toBe('alert');
	});

	it('goes to Conflicts without ending or answering anything', async () => {
		await running(conflicted());

		click(button('Open in Conflicts'));
		await settle(0);

		expect(goto).toHaveBeenCalledWith('/conflicts');
		expect(answer).not.toHaveBeenCalled();
	});

	it('takes the merge to Merger: the other branch into the one the run is on', async () => {
		await running(conflicted());
		const inMerger = conflicted();
		inMerger.run.stop = { ...inMerger.run.stop!, inMerger: true, choices: ['continue', 'end'] };
		answer.mockResolvedValue(inMerger);

		click(button('Open in Merger'));
		await settle(0);

		expect(answer).toHaveBeenCalledWith('openMerger');
		expect(merger.pickA).toHaveBeenCalledWith('main');
		expect(merger.pickB).toHaveBeenCalledWith('feature/avatars');
		expect(goto).toHaveBeenCalledWith('/merge');
		expect(said('.stop')).toContain('The merge was aborted here, to be made in Merger.');
		expect(view!.all('.stop .choices button').map((choice) => choice.textContent?.trim())).toEqual([
			'It is merged: go on',
			'End the run'
		]);
	});

	it('says why a choice cannot be taken yet, and goes on waiting', async () => {
		await running(conflicted());
		answer.mockRejectedValue('1 file is still conflicted.');

		click(button('It is resolved: go on'));
		await settle(0);

		expect(said('.stop .refused')).toBe('1 file is still conflicted.');
		expect(said('.head .status')).toBe('Waiting for you · step 3 of 6');
	});

	it('asks before each step at Ask every step, with the command it is about to run', async () => {
		const waiting = runOf(SYNC, 0);
		waiting.steps[0].state = 'waiting';
		waiting.stop = {
			step: 0,
			kind: 'approve',
			title: 'Before step 1',
			text: 'Stash — include untracked',
			paths: [],
			explained: null,
			choices: ['continue', 'end'],
			operation: null,
			source: null,
			inMerger: false
		};
		await running(viewOf(waiting));
		answer.mockResolvedValue(viewOf(runOf(SYNC, 1), { working: true }));

		expect(said('.stop')).toContain('Before step 1');
		expect(said('.stop')).toContain('git stash push -u');
		click(button('Run step 1'));
		await settle(0);

		expect(answer).toHaveBeenCalledWith('continue');
		expect(view!.find('.stop')).toBeNull();
		expect(said('.head .status')).toBe('Running · step 2 of 6');
	});

	it('shows what the agent said went wrong, in place of the output', async () => {
		const failed = runOf({ ...SYNC, agentHelp: true, agent: 'claude' }, 4);
		failed.steps[4].state = 'waiting';
		failed.stop = {
			step: 4,
			kind: 'failed',
			title: 'Commit failed: exit 1',
			text: 'nothing to commit, working tree clean',
			paths: [],
			explained: 'There is nothing to commit: the merge brought in no changes of yours.',
			choices: ['retry', 'skip', 'end'],
			operation: null,
			source: null,
			inMerger: false
		};
		failed.lines.push({ step: 4, kind: 'agent', text: 'There is nothing to commit.' });
		await running(viewOf(failed));

		expect(said('.stop .explained')).toBe('There is nothing to commit: the merge brought in no changes of yours.');
		expect(view!.all('.stop .choices button').map((choice) => choice.textContent?.trim())).toEqual([
			'Run it again',
			'Skip this step',
			'End the run'
		]);
		expect(said('.who .agent')).toBe('Claude Code');
		expect(view!.all('.output .line.agent')).toHaveLength(1);
	});

	it('follows lines as they are printed', async () => {
		await running(viewOf(runOf(SYNC, 0), { working: true }));

		heard['recipe-line']?.({
			payload: { repo: '/repos/thing', id: '1800000000-ab12', line: { step: 0, kind: 'command', text: '$ git stash push -u' } }
		});
		flushSync();

		expect(view!.all('.output .line').at(-1)?.textContent).toBe('$ git stash push -u');
	});

	it('stops a run from its header, and cannot leave one that is not over', async () => {
		await running(viewOf(runOf(SYNC, 1), { working: true }));

		expect(button('‹ Recipes').disabled).toBe(true);
		click(button('Stop'));
		await settle(0);

		expect(api.stop).toHaveBeenCalledTimes(1);
	});

	it('names the restore point, and undoes the whole run from it after asking', async () => {
		await running(conflicted());
		undo.mockResolvedValue(viewOf(runOf(SYNC, 2, { outcome: 'undone' })));

		expect(said('.restore')).toContain('refs/spagitty/restore/1800000000-ab12');
		expect(said('.restore')).toContain('HEAD · branch tips · stash · working tree');
		click(button('Undo the whole run'));
		await settle(0);

		expect(confirm.mock.calls[0][0].title).toBe('Undo the whole run?');
		expect(undo).toHaveBeenCalledTimes(1);
		expect(said('.head .status')).toBe('Undone');
		expect(said('.stop')).toContain('The run was undone');
		expect(button('Undo the whole run').disabled).toBe(true);
	});

	it('says when every step ran, and goes back to the recipe', async () => {
		await running(viewOf(runOf(SYNC, 5), { working: true }));
		heard['recipe-run']?.({ payload: viewOf(runOf(SYNC, 6, { outcome: 'done', ended: 1_800_000_030 })) });
		flushSync();

		expect(said('.head .status')).toBe('Done');
		expect(said('.stop')).toContain('Every step ran');
		expect(button('Stop').disabled).toBe(true);
		click(button('Back to the recipe'));
		await settle(0);

		expect(api.close).toHaveBeenCalledTimes(1);
		expect(view!.find('.catalogue')).not.toBeNull();
		expect(stack()).toHaveLength(6);
	});

	it('says a run that was ended keeps the steps that ran', async () => {
		await running(viewOf(runOf(SYNC, 2), { working: true }));
		heard['recipe-run']?.({ payload: viewOf(runOf(SYNC, 2, { outcome: 'ended' })) });
		flushSync();

		expect(said('.head .status')).toBe('Ended at step 3 of 6');
		expect(said('.stop')).toContain('The steps that ran are kept. The restore point is still there.');
	});
});
