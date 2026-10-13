// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Investigate, mounted (FEAT-132): one life for each of the four targets, the
 * selected event, and the ways in.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import {
	BRANCH,
	BRANCH_FACTS,
	FILE,
	FOLDER,
	HUNK,
	LINE,
	sha,
	TODAY
} from '../../testing/life-fixtures';
import type { LifeAsk, LifeEntry, RepoInfo } from '$lib/types';

const { goto, want, page } = vi.hoisted(() => ({
	goto: vi.fn(),
	want: vi.fn(),
	page: { url: new URL('http://localhost/investigate') }
}));
vi.mock('$app/navigation', () => ({ goto }));
vi.mock('$app/state', () => ({ page }));
vi.mock('$lib/graph/store.svelte', () => ({ graph: { want } }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/branches/store.svelte', async () => await import('../../testing/branches-store.svelte'));

vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => false),
	investigateLife: vi.fn(),
	investigateToday: vi.fn(),
	investigateHunk: vi.fn(),
	investigateCompare: vi.fn(),
	investigateBranch: vi.fn(),
	filesFind: vi.fn()
}));

import * as api from '$lib/api';
import { investigate } from '$lib/investigate/store.svelte';
import History from '../history/+page.svelte';
import Investigate from './+page.svelte';

const life = vi.mocked(api.investigateLife);
const today = vi.mocked(api.investigateToday);
const hunk = vi.mocked(api.investigateHunk);
const compare = vi.mocked(api.investigateCompare);
const branch = vi.mocked(api.investigateBranch);
const find = vi.mocked(api.filesFind);

const LIVES: Record<string, LifeEntry[]> = { file: FILE, folder: FOLDER, branch: BRANCH, line: LINE };

function info(branchName: string | null = 'main'): RepoInfo {
	return {
		path: '/repos/thing',
		name: 'thing',
		bare: false,
		lastFetched: null,
		head: { branch: branchName, detached: branchName === null, id: 'a'.repeat(40), short: 'aaaaaaa' }
	};
}

async function settle(): Promise<void> {
	for (let i = 0; i < 8; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	flushSync();
}

let view: Mounted | null = null;

async function at(search: string): Promise<Mounted> {
	page.url = new URL(`http://localhost/investigate${search}`);
	view = render(Investigate, {});
	await settle();
	return view;
}

const said = (selector: string) => view!.get(selector).textContent?.replace(/\s+/g, ' ').trim() ?? '';
const events = () => view!.all('.event');
const eventNamed = (text: string) => events().find((row) => row.textContent?.includes(text))!;
const button = (name: string) => view!.all('button').find((b) => b.textContent?.trim() === name)!;

beforeEach(() => {
	vi.clearAllMocks();
	investigate.clearState();
	control.reset();
	control.setInfo(info());
	life.mockImplementation(async (ask: LifeAsk) => {
		const all = LIVES[ask.target.kind];
		const entries = all.slice(ask.skip, ask.skip + ask.limit);
		return { entries, done: ask.skip + entries.length >= all.length };
	});
	today.mockResolvedValue(TODAY);
	hunk.mockResolvedValue(HUNK);
	compare.mockResolvedValue([HUNK]);
	branch.mockResolvedValue(BRANCH_FACTS);
	find.mockResolvedValue({ paths: [], more: false });
});

afterEach(() => {
	vi.useRealTimers();
	view?.destroy();
	view = null;
});

describe('a file', () => {
	beforeEach(async () => {
		await at('?kind=file&path=core/avatars.rs');
	});

	it('is asked for by the address, and the header shows it', () => {
		expect(life.mock.calls[0][0].target).toEqual({ kind: 'file', path: 'core/avatars.rs' });
		expect(view!.get('[role="radio"][aria-checked="true"]').textContent).toBe('File');
		expect((view!.get('.target input') as HTMLInputElement).value).toBe('core/avatars.rs');
	});

	it('says when it was born and by whom, and counts its commits, authors, renames and lines', () => {
		const facts = said('.facts');

		expect(facts).toMatch(/^Born \d+ [A-Z][a-z]{2}( \d{4})? · Mahmoud · /);
		expect(facts).toContain('6 commits');
		expect(facts).toContain('4 authors');
		expect(facts).toContain('2 renames');
		expect(facts).toContain('21 lines today · peak 37');
	});

	it('draws the size with a marker on born, renamed, moved and the rewrite', () => {
		expect(view!.get('.plot svg').getAttribute('aria-label')).toBe(
			'Lines over time: 18 at first, 37 at most, 21 now'
		);
		expect(view!.all('.mark span').map((mark) => mark.textContent)).toEqual([
			'born',
			'renamed',
			'moved',
			'rewrite −81%'
		]);
	});

	it('gives each author a row of ticks, the agent with the hexagon', () => {
		const rows = view!.all('.life .who');

		expect(rows.map((row) => row.querySelector('.name')?.textContent)).toEqual(['Mahmoud', 'Sara', 'Omar', 'Codex']);
		expect(view!.all('.track').map((track) => track.querySelectorAll('span').length)).toEqual([3, 1, 1, 1]);
		expect(rows[3].querySelector('svg')).not.toBeNull();
		expect(rows[3].style.color).toBe('var(--agent)');
		expect(rows[0].querySelector('svg')).toBeNull();
	});

	it('lists the events newest first, with kind, who, summary and lines', () => {
		expect(events().map((row) => row.querySelector('.kind')?.textContent)).toEqual([
			'edit',
			'rewrite',
			'moved',
			'renamed',
			'edit',
			'born'
		]);
		const rewrite = eventNamed('Expire avatars daily');
		expect(rewrite.querySelector('.who')?.textContent?.trim()).toBe('Omar');
		expect(rewrite.querySelector('.lines')?.textContent?.replace(/\s+/g, ' ').trim()).toBe('+31 −52');
		// A rename is read by its two names.
		expect(eventNamed('avatar.rs → avatars.rs')).toBeDefined();
	});

	it('shows the newest event at first, and the one pressed after', async () => {
		expect(events()[0].getAttribute('aria-pressed')).toBe('true');
		expect(said('.detail .head')).toContain('Cache avatars on disk');

		click(eventNamed('Expire avatars daily'));
		await settle();

		expect(said('.detail .head')).toContain('rewrite Expire avatars daily');
		expect(said('.detail .head')).toContain('Omar');
	});

	it('says how much of the file the event changed, what survives, and who owns it today', async () => {
		click(eventNamed('Expire avatars daily'));
		await settle();

		const stats = said('.detail .stats');
		expect(stats).toContain('81% of the file changed');
		// Five of the thirty-one lines it wrote are still there.
		expect(stats).toContain('5 / 31 of its lines still alive');
		expect(stats).toContain('Who owns it today');
		expect(view!.get('.bar').getAttribute('aria-label')).toBe('Mahmoud 43%, Omar 38%, Codex 19%');
		expect(view!.all('.bar span').map((part) => part.style.width)).toEqual(['43%', '38%', '19%']);
		expect(view!.all('.bar span')[2].style.background).toBe('var(--agent)');
	});

	it('shows the event’s largest hunk', async () => {
		click(eventNamed('Expire avatars daily'));
		await settle();

		expect(hunk).toHaveBeenLastCalledWith(sha(5), 'core/avatars.rs', null);
		expect(said('.hunks .at')).toBe('@@ -4,3 +4,4 @@ pub fn avatar');
		expect(view!.all('.hunks .row').map((row) => row.className.replace('row ', '').split(' ')[0])).toEqual([
			'context',
			'removed',
			'added',
			'added'
		]);
	});

	it('says so when the event changed no line', async () => {
		hunk.mockResolvedValue(null);
		click(eventNamed('avatar.rs → avatars.rs'));
		await settle();

		expect(said('.hunks')).toBe('No line of the file changed');
	});

	it('opens the file in Files as the commit left it, under the name it had', async () => {
		click(eventNamed('avatar.rs → avatars.rs'));
		await settle();

		click(view!.all('.actions button')[0]);

		expect(view!.all('.actions button')[0].textContent?.replace(/\s+/g, ' ').trim()).toBe(
			`Open file at ${sha(3).slice(-7)}`
		);
		const went = new URL(`http://x${goto.mock.calls.at(-1)?.[0]}`);
		expect(went.pathname).toBe('/files');
		expect(Object.fromEntries(went.searchParams)).toEqual({ rev: sha(3), path: 'src/avatars.rs' });
	});

	it('compares the file then with the file today, and goes back', async () => {
		click(eventNamed('Key avatars by size'));
		await settle();

		click(button('Compare with today'));
		await settle();
		expect(compare).toHaveBeenCalledWith(sha(2), 'src/avatar.rs', '', 'core/avatars.rs');
		expect(view!.get('.hunks').getAttribute('aria-label')).toBe('This commit against today');

		click(button('Back to this commit'));
		expect(view!.get('.hunks').getAttribute('aria-label')).toBe('Largest change');
	});

	it('shows the commit on the Graph', () => {
		click(button('Show in Graph'));

		expect(want).toHaveBeenCalledWith(sha(6));
		expect(goto).toHaveBeenCalledWith('/');
	});

	it('moves through the events with the arrows', async () => {
		press(view!.get('.events'), 'ArrowDown');
		await settle();

		expect(events()[1].getAttribute('aria-pressed')).toBe('true');
		press(view!.get('.events'), 'ArrowUp');
		expect(events()[0].getAttribute('aria-pressed')).toBe('true');
	});

	it('reads again without following renames', async () => {
		const chip = view!.all('button').find((b) => b.textContent?.trim() === 'Follow renames')!;
		expect(chip.classList.contains('active')).toBe(true);

		click(chip);
		await settle();

		expect(life.mock.calls.at(-1)?.[0].follow).toBe(false);
	});

	it('reads as far back as the range chosen', async () => {
		click(view!.get('.range'));
		await settle();
		const items = view!.all('[role="menuitem"]');
		expect(items.map((item) => item.querySelector('.label')?.textContent)).toEqual([
			'All time',
			'The last year',
			'The last 90 days',
			'The last 30 days'
		]);

		click(items[2]);
		await settle();

		expect(life.mock.calls.at(-1)?.[0].since).toBeGreaterThan(0);
		expect(said('.range')).toContain('The last 90 days');
	});
});

describe('a folder', () => {
	beforeEach(async () => {
		await at('?kind=folder&path=core');
	});

	it('counts its files and lines, and says where it was born', () => {
		const facts = said('.facts');

		expect(facts).toContain('Born');
		expect(facts).toContain('13 files today');
		expect(facts).toContain(`${FOLDER.length} commits`);
		expect(facts).not.toContain('renames');
	});

	it('lists what added, removed or renamed a file, and its biggest edits', () => {
		const kinds = events().map((row) => row.querySelector('.kind')?.textContent);

		expect(kinds.slice(0, 3)).toEqual(['added', 'removed', 'renamed']);
		expect(kinds.filter((kind) => kind === 'edit').length).toBeLessThan(30);
		expect(kinds).toContain('edit');
	});

	it('says how many files the event added, removed and renamed, and offers no comparison', () => {
		expect(said('.detail .stats')).toContain('3 · 0 · 0 files added · removed · renamed');
		expect(view!.find('.hunks')).toBeNull();
		expect(view!.all('.actions button').map((b) => b.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			`Open folder at ${sha(40).slice(-7)}`,
			'Show in Graph'
		]);
		expect(today).not.toHaveBeenCalled();

		// Files opens its tree at that folder, as the commit left it.
		click(view!.all('.actions button')[0]);
		const went = new URL(`http://x${goto.mock.calls.at(-1)?.[0]}`);
		expect(Object.fromEntries(went.searchParams)).toEqual({ rev: sha(40), dir: 'core' });
	});

	it('has no switch for following renames: a folder is not followed', () => {
		expect(view!.all('button').some((b) => b.textContent?.trim() === 'Follow renames')).toBe(false);
	});
});

describe('a branch', () => {
	it('is the branch checked out, when nothing is asked for', async () => {
		await at('');

		expect(life.mock.calls[0][0].target).toEqual({ kind: 'branch', name: 'main' });
		expect(view!.get('[role="radio"][aria-checked="true"]').textContent).toBe('Branch');
		expect(said('.head .target')).toContain('main');
	});

	it('says where it left the other branch, and how it stands against it today', async () => {
		await at('?kind=branch&name=feature/avatars');
		const facts = said('.facts');

		expect(facts).toContain('from main');
		expect(facts).toContain('3 commits');
		expect(facts).toContain('0 ahead of main · 3 behind');
		expect(branch).toHaveBeenCalledWith('feature/avatars');
	});

	it('lists its commits, the merges into it, where it landed and what its reflog says', async () => {
		await at('?kind=branch&name=feature/avatars');

		expect(events().map((row) => row.querySelector('.kind')?.textContent)).toEqual([
			'merged',
			'commit',
			'rebase',
			'merge',
			'commit',
			'born'
		]);
		expect(view!.all('.mark span').map((mark) => mark.textContent)).toEqual(['born', 'merged']);
		expect(view!.get('.plot svg').getAttribute('aria-label')).toContain('Commits over time');
	});

	it('offers only the Graph: a branch is not one file', async () => {
		await at('?kind=branch&name=feature/avatars');

		expect(view!.all('.actions button').map((b) => b.textContent?.trim())).toEqual(['Show in Graph']);
		expect(said('.detail .head')).toContain('into main');
	});

	it('is chosen from the branch menu', async () => {
		await at('?kind=branch&name=feature/avatars');

		click(view!.get('.head .target'));
		await settle();

		expect(view!.find('.branch-menu')).not.toBeNull();
		// Reading a branch's life checks nothing out, so there is no "A commit…" here either.
		expect(view!.find('.branch-menu .commit')).toBeNull();
	});
});

describe('a line', () => {
	beforeEach(async () => {
		await at(`?kind=line&path=core/avatars.rs&line=14&rev=${sha(9)}`);
	});

	it('is asked for with its file, its number and the revision Files was showing', () => {
		expect(life.mock.calls[0][0]).toMatchObject({
			target: { kind: 'line', path: 'core/avatars.rs', line: 14 },
			rev: sha(9)
		});
		expect((view!.get('.target input.number') as HTMLInputElement).value).toBe('14');
	});

	it('has no size to draw, and still says who and when', () => {
		expect(view!.find('.plot')).toBeNull();
		expect(said('.facts')).toContain('3 commits');
		expect(said('.facts')).not.toContain('lines today');
		expect(view!.all('.life .who .name').map((name) => name.textContent)).toEqual(['Mahmoud', 'Omar', 'Codex']);
	});

	it('shows the line as each commit left it', async () => {
		expect(said('.then')).toBe('fs::write(&path, &bytes)?;');

		click(eventNamed('Add avatar cache'));
		await settle();

		expect(said('.then')).toBe('MEMORY.insert(key, bytes);');
		expect(said('.detail .head')).toContain('born');
	});
});

describe('choosing what to investigate', () => {
	it('goes from a file to its line, and to the folder it is in', async () => {
		await at('?kind=file&path=core/avatars.rs');

		click(view!.all('[role="radio"]')[3]);
		await settle();
		expect(life.mock.calls.at(-1)?.[0].target).toEqual({ kind: 'line', path: 'core/avatars.rs', line: 1 });

		click(view!.all('[role="radio"]')[1]);
		await settle();
		expect(life.mock.calls.at(-1)?.[0].target).toEqual({ kind: 'folder', path: 'core' });
	});

	/** The repository is read again after any commit, and that is not a new address. */
	it('keeps the target the person chose when the repository is read again', async () => {
		await at('?kind=file&path=core/avatars.rs');
		click(view!.all('[role="radio"]')[1]);
		await settle();
		expect(investigate.target).toEqual({ kind: 'folder', path: 'core' });
		life.mockClear();

		// The same repository, as a refresh hands it over: a new object.
		control.setInfo({ ...info(), lastFetched: 1_800_000_000 });
		await settle();

		expect(investigate.target).toEqual({ kind: 'folder', path: 'core' });
		expect(life.mock.calls.every((call) => call[0].target.kind === 'folder')).toBe(true);
	});

	it('applies the address again for another repository', async () => {
		await at('?kind=file&path=core/avatars.rs');
		click(view!.all('[role="radio"]')[1]);
		await settle();

		control.setInfo({ ...info(), path: '/repos/another', name: 'another' });
		await settle();

		expect(investigate.target).toEqual({ kind: 'file', path: 'core/avatars.rs' });
	});

	it('waits for a path when there is none to go by', async () => {
		await at('?kind=branch&name=main');
		life.mockClear();

		click(view!.all('[role="radio"]')[0]);
		await settle();

		expect(life).not.toHaveBeenCalled();
		expect((view!.get('.target input') as HTMLInputElement).value).toBe('');
	});

	it('takes a typed path on Enter', async () => {
		await at('?kind=branch&name=main');
		click(view!.all('[role="radio"]')[0]);
		await settle();
		const field = view!.get('.target input') as HTMLInputElement;

		field.value = '/src/lib.rs';
		fire(field, 'input');
		press(field, 'Enter');
		await settle();

		expect(life.mock.calls.at(-1)?.[0].target).toEqual({ kind: 'file', path: 'src/lib.rs' });
	});

	it('offers the paths at the revision that hold what is typed', async () => {
		find.mockResolvedValue({ paths: ['src/lib.rs', 'src/deep/lib.rs'], more: false });
		await at('?kind=file&path=core/avatars.rs');
		const field = view!.get('.target input') as HTMLInputElement;

		// Every letter is a walk of the tree, so it waits for a pause.
		vi.useFakeTimers();
		field.value = 'lib';
		fire(field, 'input');
		await vi.advanceTimersByTimeAsync(200);
		vi.useRealTimers();
		await settle();

		const offered = view!.all('.suggestions [role="option"]');
		expect(offered.map((option) => option.textContent)).toEqual(['src/lib.rs', 'src/deep/lib.rs']);

		press(field, 'ArrowDown');
		press(field, 'ArrowDown');
		press(field, 'Enter');
		await settle();
		expect(life.mock.calls.at(-1)?.[0].target).toEqual({ kind: 'file', path: 'src/deep/lib.rs' });
	});

	it('offers a folder the folders the paths are in', async () => {
		find.mockResolvedValue({ paths: ['crates/core/src/lib.rs', 'crates/core/Cargo.toml', 'docs/core.md'], more: false });
		await at('?kind=folder&path=');
		const field = view!.get('.target input') as HTMLInputElement;

		vi.useFakeTimers();
		field.value = 'core';
		fire(field, 'input');
		await vi.advanceTimersByTimeAsync(200);
		vi.useRealTimers();
		await settle();

		expect(view!.all('.suggestions [role="option"]').map((option) => option.textContent)).toEqual([
			'crates/core',
			'crates/core/src'
		]);
	});
});

describe('with nothing to investigate', () => {
	it('says to open a repository when none is open', async () => {
		control.setInfo(null);
		await at('');

		expect(view!.text()).toContain('Open a repository to investigate it');
		expect(life).not.toHaveBeenCalled();
	});

	it('asks what to investigate when nothing is checked out by name', async () => {
		control.setInfo(info(null));
		await at('');

		expect(view!.text()).toContain('Choose a file, a folder, a branch or a line');
		expect(life).not.toHaveBeenCalled();
	});

	it('says why a life could not be read', async () => {
		life.mockRejectedValue('no commit nowhere');
		await at('?kind=file&path=a.rs&rev=nowhere');

		expect(view!.get('.events').textContent).toContain('no commit nowhere');
	});

	it('says when the range holds nothing', async () => {
		life.mockResolvedValue({ entries: [], done: true });
		await at('?kind=file&path=a.rs');

		expect(view!.get('.events').textContent).toContain('Nothing in this range');
		expect(said('.detail')).toBe('Choose an event');
	});
});

/** File history became Investigate, and its address goes there with its path. */
describe('/history', () => {
	it('goes to Investigate with the path it was given', async () => {
		page.url = new URL('http://localhost/history?path=src/my%20file.rs');
		const old = render(History, {});
		await settle();

		expect(goto).toHaveBeenCalledWith('/investigate?kind=file&path=src%2Fmy+file.rs', { replaceState: true });
		old.destroy();
	});

	it('goes to Investigate with nothing, when it was given nothing', async () => {
		page.url = new URL('http://localhost/history');
		const old = render(History, {});
		await settle();

		expect(goto).toHaveBeenCalledWith('/investigate', { replaceState: true });
		old.destroy();
	});
});
