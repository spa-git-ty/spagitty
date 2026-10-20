// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Untrack (FEAT-155): choosing what to untrack on a branch, what the commit
 * will say and add, and the commit and push it asks for.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render, type Mounted } from '../../testing/mount';
import type { FilesEntry } from '$lib/types';

const { push, info } = vi.hoisted(() => ({
	push: vi.fn(),
	info: {
		value: { path: '/repos/thing', name: 'thing', bare: false, head: { branch: 'main' } } as unknown
	}
}));

vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	inTauri: vi.fn(() => true),
	filesResolve: vi.fn(),
	filesTree: vi.fn(),
	filesFind: vi.fn(),
	untrack: vi.fn()
}));
vi.mock('$lib/network/store.svelte', () => ({ network: { push } }));
vi.mock('$lib/repo.svelte', () => ({
	repo: {
		get info() {
			return info.value;
		},
		generation: 1
	}
}));
vi.mock('$lib/branches/store.svelte', () => ({
	branches: {
		rows: [
			{ name: 'main', kind: 'branch', current: true },
			{ name: 'release', kind: 'branch', current: false },
			{ name: 'origin/main', kind: 'remote', current: false }
		],
		load: vi.fn()
	}
}));

import * as api from '$lib/api';
import UntrackScreen from './UntrackScreen.svelte';
import { ignoreLine, suggested, untrack } from './store.svelte';

const resolve = vi.mocked(api.filesResolve);
const tree = vi.mocked(api.filesTree);
const commit = vi.mocked(api.untrack);

function entry(path: string, kind: FilesEntry['kind'] = 'file'): FilesEntry {
	return { path, name: path.slice(path.lastIndexOf('/') + 1), kind };
}

async function settle(): Promise<void> {
	for (let i = 0; i < 5; i++) {
		await Promise.resolve();
		await new Promise((done) => setTimeout(done, 0));
	}
	flushSync();
}

let view: Mounted | null = null;

beforeEach(() => {
	vi.clearAllMocks();
	untrack.clearState();
	resolve.mockImplementation(async (rev) => ({ id: `${rev}-id`, short: rev }));
	tree.mockImplementation(async (_rev, dir) =>
		dir === ''
			? [entry('build', 'folder'), entry('secrets.env'), entry('README.md')]
			: [entry(`${dir}/out.js`), entry(`${dir}/map.js`)]
	);
	commit.mockResolvedValue({ commit: 'c0ffee1234', branch: 'main', remote: 'origin', hooksRan: true });
	push.mockResolvedValue(true);
});

afterEach(() => {
	view?.destroy();
	view = null;
});

describe('what the commit will say and add', () => {
	it('suggests a message naming what is untracked', () => {
		expect(suggested([])).toBe('');
		expect(suggested([{ path: 'secrets.env', folder: false }])).toBe('Stop tracking secrets.env');
		expect(
			suggested([
				{ path: 'secrets.env', folder: false },
				{ path: 'build', folder: true }
			])
		).toBe('Stop tracking 2 paths\n\n- secrets.env\n- build/');
	});

	it('shows the .gitignore lines untrack.rs writes', () => {
		expect(ignoreLine({ path: 'a/b.txt', folder: false })).toBe('/a/b.txt');
		expect(ignoreLine({ path: 'node_modules', folder: true })).toBe('/node_modules/');
		expect(ignoreLine({ path: 'weird[1]*.txt', folder: false })).toBe('/weird\\[1\\]\\*.txt');
		expect(ignoreLine({ path: 'space ', folder: false })).toBe('/space\\ ');
	});
});

describe('choosing', () => {
	it('reads the branch’s own tree, and a folder takes in what was chosen inside it', async () => {
		await untrack.choose('release');
		expect(resolve).toHaveBeenCalledWith('release');
		expect(tree).toHaveBeenCalledWith('release-id', '');

		untrack.toggle({ path: 'build/out.js', folder: false });
		untrack.toggle({ path: 'build', folder: true });
		expect(untrack.chosen).toEqual([{ path: 'build', folder: true }]);
		expect(untrack.isChosen('build/map.js')).toBe(true);
		expect(untrack.isCovered('build/map.js')).toBe(true);

		// Inside a chosen folder, a file cannot be chosen apart from it.
		untrack.toggle({ path: 'build/map.js', folder: false });
		expect(untrack.chosen).toHaveLength(1);
	});

	it('lets go of what was chosen when the branch changes', async () => {
		await untrack.choose('main');
		untrack.toggle({ path: 'secrets.env', folder: false });
		await untrack.choose('release');
		expect(untrack.chosen).toEqual([]);
	});

	it('keeps a typed message over the suggested one', async () => {
		await untrack.choose('main');
		untrack.toggle({ path: 'secrets.env', folder: false });
		untrack.setMessage('Keep secrets out of git');
		untrack.toggle({ path: 'build', folder: true });
		expect(untrack.message).toBe('Keep secrets out of git');
	});
});

describe('untracking', () => {
	it('commits what is chosen, then pushes the branch to its remote', async () => {
		await untrack.choose('main');
		untrack.toggle({ path: 'secrets.env', folder: false });

		expect(await untrack.run()).toBe(true);
		expect(commit).toHaveBeenCalledWith(
			'main',
			[{ path: 'secrets.env', folder: false }],
			true,
			'Stop tracking secrets.env'
		);
		expect(push).toHaveBeenCalledWith('origin', 'main');
		expect(untrack.done?.commit).toBe('c0ffee1234');
		expect(untrack.pushed).toBe(true);
		expect(untrack.chosen).toEqual([]);
	});

	it('pushes nothing when asked not to, or when there is no remote', async () => {
		await untrack.choose('main');
		untrack.toggle({ path: 'secrets.env', folder: false });
		untrack.setPush(false);
		await untrack.run();
		expect(push).not.toHaveBeenCalled();

		commit.mockResolvedValue({ commit: 'abc', branch: 'main', remote: null, hooksRan: true });
		untrack.setPush(true);
		untrack.toggle({ path: 'README.md', folder: false });
		await untrack.run();
		expect(push).not.toHaveBeenCalled();
	});

	it('says why a commit was refused, and keeps what was chosen', async () => {
		commit.mockRejectedValue('1 change is already staged here');
		await untrack.choose('main');
		untrack.toggle({ path: 'secrets.env', folder: false });

		expect(await untrack.run()).toBe(false);
		expect(untrack.error).toBe('1 change is already staged here');
		expect(untrack.chosen).toHaveLength(1);
	});
});

describe('the screen', () => {
	it('starts on the branch checked out, and offers local branches only', async () => {
		view = render(UntrackScreen, {});
		await settle();

		expect(resolve).toHaveBeenCalledWith('main');
		expect(view.get('#untrack-branch').textContent).toContain('main');
		expect(view.text()).toContain('Checked out here');
		expect(view.all('.rows .name').map((name) => name.textContent)).toEqual(['build', 'secrets.env', 'README.md']);
	});

	it('shows what will be untracked, its .gitignore lines and message, before anything is written', async () => {
		view = render(UntrackScreen, {});
		await settle();

		const boxes = view.all('.rows input[type="checkbox"]') as HTMLInputElement[];
		fire(boxes[1], 'change');
		click(view.get('.rows .arrow') as HTMLElement);
		await settle();
		fire(view.all('.rows input[type="checkbox"]')[0], 'change');

		expect(view.all('.chosen .path').map((path) => path.textContent)).toEqual(['secrets.env', 'build/']);
		expect(view.get('.lines').textContent).toBe('/secrets.env\n/build/');
		expect((view.get('#untrack-message') as HTMLTextAreaElement).value).toBe(
			'Stop tracking 2 paths\n\n- secrets.env\n- build/'
		);
		// A file inside the chosen folder is ticked with it, and cannot be let go alone.
		const inner = view.all('.rows input[type="checkbox"]')[1] as HTMLInputElement;
		expect(inner.checked).toBe(true);
		expect(inner.disabled).toBe(true);
		expect(commit).not.toHaveBeenCalled();
		expect(view.get('.go button').textContent?.trim()).toBe('Untrack, commit and push');
	});

	it('commits and says so', async () => {
		view = render(UntrackScreen, {});
		await settle();
		fire(view.all('.rows input[type="checkbox"]')[1], 'change');

		click(view.get('.go button') as HTMLElement);
		await settle();

		expect(commit).toHaveBeenCalled();
		expect(view.get('[role="status"]').textContent?.replace(/\s+/g, ' ').trim()).toBe(
			'Committed c0ffee1 on main. Pushing to origin.'
		);
	});
});
