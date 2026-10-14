// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Terminal, mounted (FEAT-130), with an emulator that records and a PTY that
 * is a channel the test holds the other end of.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, flushSync, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import {
	emulators,
	PLAIN,
	PSMUX,
	SESSIONS,
	settle as wait,
	TMUX,
	WINDOWS,
	type Opened
} from '../../testing/terminal-fixtures';
import type { RepoInfo } from '$lib/types';

const { goto, want, exits } = vi.hoisted(() => ({
	goto: vi.fn(),
	want: vi.fn(),
	exits: { handler: null as null | ((event: { payload: { id: number; code: number | null } }) => void) }
}));
vi.mock('$app/navigation', () => ({ goto }));
vi.mock('$lib/graph/store.svelte', () => ({ graph: { want } }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
// The emulator is xterm.js, which wants a canvas. The screen is tested with
// one that records what it was given.
vi.mock('$lib/terminal/xterm', async () => ({
	makeXterm: (await import('../../testing/terminal-fixtures')).makeRecorded
}));
vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (_name: string, handler: (typeof exits)['handler']) => {
		exits.handler = handler;
		return () => {};
	})
}));

vi.mock('$lib/api', async (importOriginal) => ({
	...(await importOriginal<typeof import('$lib/api')>()),
	terminalInfo: vi.fn(),
	terminalOpen: vi.fn(),
	terminalWrite: vi.fn(async () => {}),
	terminalResize: vi.fn(async () => {}),
	terminalClose: vi.fn(async () => {}),
	terminalSessionName: vi.fn(),
	terminalSessions: vi.fn(),
	terminalWindows: vi.fn(),
	terminalAct: vi.fn(),
	terminalLinks: vi.fn()
}));

import * as api from '$lib/api';
import { terminal } from '$lib/terminal/store.svelte';
import Terminal from './+page.svelte';

const info = vi.mocked(api.terminalInfo);
const open = vi.mocked(api.terminalOpen);
const write = vi.mocked(api.terminalWrite);
const resize = vi.mocked(api.terminalResize);
const close = vi.mocked(api.terminalClose);
const sessionName = vi.mocked(api.terminalSessionName);
const sessions = vi.mocked(api.terminalSessions);
const windows = vi.mocked(api.terminalWindows);
const act = vi.mocked(api.terminalAct);
const links = vi.mocked(api.terminalLinks);

function repository(name = 'thing'): RepoInfo {
	return {
		path: `/repos/${name}`,
		name,
		bare: false,
		lastFetched: null,
		head: { branch: 'main', detached: false, id: 'a'.repeat(40), short: 'aaaaaaa' }
	};
}

let opened: Opened[] = [];
let view: Mounted | null = null;

async function settle(): Promise<void> {
	await wait();
	flushSync();
}

async function show(): Promise<Mounted> {
	view = render(Terminal, {});
	await settle();
	return view;
}

const said = (selector: string) => view!.get(selector).textContent?.replace(/\s+/g, ' ').trim() ?? '';
const tabs = () => view!.all('.tabs [role="tab"]');
const labelled = (label: string) => view!.find(`[aria-label="${label}"]`);
const button = (name: string) => view!.all('button').find((b) => b.textContent?.trim() === name);
const exit = (id: number) => exits.handler?.({ payload: { id, code: 0 } });

beforeEach(() => {
	vi.clearAllMocks();
	localStorage.clear();
	terminal.clearState();
	control.reset();
	control.setInfo(repository());
	emulators.length = 0;
	opened = [];
	exits.handler = null;

	info.mockResolvedValue(TMUX);
	open.mockImplementation(async (ask, output) => {
		const id = opened.length + 1;
		opened.push({ id, ask, print: (text) => output(new TextEncoder().encode(text)) });
		return id;
	});
	sessionName.mockImplementation(async (path) => path.split(/[\\/]/).pop() || 'spagitty');
	sessions.mockResolvedValue(SESSIONS);
	windows.mockResolvedValue(WINDOWS);
	act.mockResolvedValue(undefined);
	links.mockResolvedValue({ commits: [], paths: [] });
});

afterEach(() => {
	view?.destroy();
	view = null;
});

describe('with a multiplexer', () => {
	it('shows the repository’s session: its terminal in the pane, with the focus', async () => {
		await show();

		expect(opened.map((one) => one.ask)).toEqual([
			{ session: 'thing', cwd: '/repos/thing', cols: 80, rows: 24 }
		]);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[0].element);
		expect(emulators[0].focused).toBeGreaterThan(0);
		expect(view!.get('.view').getAttribute('aria-label')).toBe('Terminal thing');
	});

	it('tells the program the size of the pane it was put in', async () => {
		await show();

		expect(emulators[0].fits).toBeGreaterThan(0);
		expect(resize).toHaveBeenCalledWith(1, 132, 40);
	});

	it('has a tab for each session, the one shown selected, and `+` for a new window', async () => {
		await show();

		expect(tabs().map((tab) => [tab.textContent?.trim(), tab.getAttribute('aria-selected')])).toEqual([
			['thing', 'true'],
			['quiblo', 'false']
		]);
		expect(tabs()[1].title).toBe('quiblo — detached');
		// A session is detached from, never closed: no tab has a way to close it.
		expect(view!.all('.shut')).toEqual([]);

		click(labelled('New window')!);
		await settle();
		expect(act).toHaveBeenCalledWith('thing', { kind: 'newWindow', cwd: '/repos/thing' });
	});

	/** The item: the pill names the multiplexer, and its hover gives the path and version. */
	it('names the system’s multiplexer in the pill, with its path and version on hover', async () => {
		await show();

		expect(said('.pill')).toBe('tmux 3.5a');
		expect(view!.get('.pill').title).toBe('/usr/bin/tmux · tmux 3.5a');
	});

	it('says *bundled* when the multiplexer is the one Spagitty ships', async () => {
		info.mockResolvedValue(PSMUX);

		await show();

		expect(said('.pill')).toBe('psmux 3.3.8 bundled');
		expect(view!.get('.pill').title).toBe('C:\\app\\multiplexer\\psmux.exe · psmux 3.3.8');
	});

	it('lists the sessions with their windows and whether anyone is attached', async () => {
		await show();

		expect(view!.all('.session').map((row) => row.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'thing 3 windows · attached',
			'quiblo 1 window · detached'
		]);
		expect(view!.all('.session').map((row) => row.getAttribute('aria-pressed'))).toEqual(['true', 'false']);
		expect(view!.all('.window').map((row) => row.textContent?.replace(/\s+/g, ' ').trim())).toEqual([
			'1·shell',
			'2·tests',
			'3·logs'
		]);
		expect(view!.all('.window').map((row) => row.getAttribute('aria-pressed'))).toEqual([
			'true',
			'false',
			'false'
		]);
		// The shell and the multiplexer, under them.
		expect(said('.panel')).toContain('zsh · tmux');
	});

	it('selects a window, splits and detaches by asking the multiplexer', async () => {
		await show();

		click(view!.all('.window')[1]);
		await settle();
		click(labelled('Split right')!);
		await settle();
		click(labelled('Split down')!);
		await settle();
		click(button('Detach')!);
		await settle();

		expect(act.mock.calls).toEqual([
			['thing', { kind: 'selectWindow', index: 2 }],
			['thing', { kind: 'splitRight' }],
			['thing', { kind: 'splitDown' }],
			['thing', { kind: 'detach' }]
		]);
	});

	it('says a session is detached once its client has ended, and attaches again', async () => {
		await show();
		sessions.mockResolvedValue([{ name: 'thing', windows: 3, attached: false }]);

		exit(1);
		await settle();

		expect(view!.get('.pane').childElementCount).toBe(0);
		expect(said('.view')).toBe('thing is detached Attach');
		expect((button('Detach') as HTMLButtonElement).disabled).toBe(true);
		expect(tabs().map((tab) => tab.textContent?.trim())).toEqual(['thing']);

		click(button('Attach')!);
		await settle();

		expect(opened.map((one) => one.ask.session)).toEqual(['thing', 'thing']);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[1].element);
		expect((button('Detach') as HTMLButtonElement).disabled).toBe(false);
	});

	it('attaches to another session from its tab and from the panel', async () => {
		await show();

		click(tabs()[1]);
		await settle();
		expect(opened.map((one) => one.ask.session)).toEqual(['thing', 'quiblo']);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[1].element);
		expect(emulators[0].disposed).toBe(false);

		click(view!.all('.session')[0]);
		await settle();
		// The client that was there, put back; not a third.
		expect(opened).toHaveLength(2);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[0].element);
	});

	it('says there is no session yet before the multiplexer lists one', async () => {
		sessions.mockResolvedValue([]);
		windows.mockResolvedValue([]);

		await show();

		expect(said('.panel')).toContain('No session yet');
		expect(view!.all('.window')).toEqual([]);
		expect(said('.panel')).not.toContain('Windows');
	});

	it('says what the multiplexer said when it refuses', async () => {
		await show();
		act.mockRejectedValue('no space for a new pane');

		click(labelled('Split right')!);
		await settle();

		expect(said('.error')).toBe('no space for a new pane');
	});
});

describe('follow repository', () => {
	it('is on, and switches to another repository’s session when that repository is opened', async () => {
		await show();
		expect(said('.follow')).toBe('Follow repository on');

		control.setInfo(repository('quiblo'));
		await settle();

		expect(opened.map((one) => [one.ask.session, one.ask.cwd])).toEqual([
			['thing', '/repos/thing'],
			['quiblo', '/repos/quiblo']
		]);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[1].element);
		expect(tabs().find((tab) => tab.getAttribute('aria-selected') === 'true')?.textContent?.trim()).toBe('quiblo');
	});

	it('stays where it is once switched off', async () => {
		await show();

		click(view!.get('.follow button'));
		expect(said('.follow')).toBe('Follow repository off');
		control.setInfo(repository('quiblo'));
		await settle();

		expect(opened).toHaveLength(1);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[0].element);
	});
});

describe('with no multiplexer', () => {
	beforeEach(() => info.mockResolvedValue(PLAIN));

	/** The item: the panel is hidden, and nothing says anything is missing. */
	it('is a plain terminal, with no Sessions panel and nothing said to be missing', async () => {
		await show();

		expect(opened.map((one) => one.ask)).toEqual([
			{ session: null, cwd: '/repos/thing', cols: 80, rows: 24 }
		]);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[0].element);
		expect(view!.find('.panel')).toBeNull();
		expect(view!.find('.pill')).toBeNull();
		expect(labelled('Split right')).toBeNull();
		expect(labelled('Split down')).toBeNull();
		expect(button('Detach')).toBeUndefined();
		expect(view!.text()).not.toMatch(/tmux|psmux|multiplexer|missing|install|session/i);
		expect(sessions).not.toHaveBeenCalled();
	});

	it('has a tab for each shell, `+` for another, and a way to close each', async () => {
		await show();

		click(labelled('New terminal')!);
		await settle();

		expect(tabs().map((tab) => [tab.textContent?.trim(), tab.getAttribute('aria-selected')])).toEqual([
			['pwsh', 'false'],
			['pwsh 2', 'true']
		]);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[1].element);

		click(tabs()[0]);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[0].element);

		click(labelled('Close pwsh 2')!);
		await settle();
		expect(close).toHaveBeenCalledWith(2);
		expect(tabs().map((tab) => tab.textContent?.trim())).toEqual(['pwsh']);
	});

	it('offers a new terminal when the last one is closed', async () => {
		await show();

		click(labelled('Close pwsh')!);
		await settle();

		expect(view!.get('.pane').childElementCount).toBe(0);
		click(button('New terminal')!);
		await settle();
		expect(opened).toHaveLength(2);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[1].element);
	});

	it('takes the tab of a shell that exited', async () => {
		await show();
		click(labelled('New terminal')!);
		await settle();

		exit(2);
		await settle();

		expect(tabs().map((tab) => tab.textContent?.trim())).toEqual(['pwsh']);
		expect(view!.get('.pane').firstElementChild).toBe(emulators[0].element);
	});

	it('says why a shell could not be started', async () => {
		open.mockRejectedValue('fish could not be started: program not found');

		await show();

		expect(said('.error')).toBe('fish could not be started: program not found');
		expect(tabs()).toEqual([]);
	});
});

describe('the PTY channel', () => {
	it('draws what the program prints, and sends the program what is typed', async () => {
		await show();

		opened[0].print('~/thing main\r\n> ');
		await settle();
		expect(emulators[0].text()).toBe('~/thing main\r\n> ');

		emulators[0].hooks.onData('git status\r');
		expect(write).toHaveBeenCalledWith(1, 'git status\r');
	});

	/** A terminal outlives the screen: leaving and coming back loses nothing. */
	it('keeps the terminal and what it printed while the screen was away', async () => {
		await show();
		const first = emulators[0];
		opened[0].print('before\r\n');
		await settle();

		view!.destroy();
		view = null;
		expect(first.disposed).toBe(false);
		opened[0].print('while away\r\n');
		await settle();

		await show();

		expect(opened).toHaveLength(1);
		expect(emulators).toHaveLength(1);
		expect(view!.get('.pane').firstElementChild).toBe(first.element);
		expect(first.text()).toBe('before\r\nwhile away\r\n');
	});
});

describe('links', () => {
	const sha = 'a3f9c1e' + '0'.repeat(33);

	beforeEach(() => {
		links.mockResolvedValue({ commits: [['a3f9c1e', sha]], paths: ['src/a.ts'] });
	});

	it('opens a commit on the Graph', async () => {
		await show();
		const [commit] = await emulators[0].hooks.links('a3f9c1e Fix the lane order');

		emulators[0].hooks.open(commit);

		expect(want).toHaveBeenCalledWith(sha);
		expect(goto).toHaveBeenCalledWith('/');
	});

	it('opens a place in a file in Files, at HEAD and on its line', async () => {
		await show();
		const [place] = await emulators[0].hooks.links('  --> src/a.ts:214:9');

		emulators[0].hooks.open(place);

		expect(goto).toHaveBeenCalledWith('/files?path=src%2Fa.ts&line=214&rev=HEAD');
		expect(want).not.toHaveBeenCalled();
	});

	it('asks again about a word once another repository is open', async () => {
		await show();
		await emulators[0].hooks.links('a3f9c1e');
		expect(links).toHaveBeenCalledTimes(1);

		terminal.setFollow(false);
		control.setInfo(repository('quiblo'));
		await settle();
		await emulators[0].hooks.links('a3f9c1e');

		expect(links).toHaveBeenCalledTimes(2);
	});
});
