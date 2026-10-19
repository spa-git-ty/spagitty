// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Terminal screen's state, with an emulator that records and a PTY that
 * is a channel the test holds the other end of (FEAT-130).
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
	emulators,
	makeRecorded,
	PLAIN,
	SESSIONS,
	settle,
	TMUX,
	WINDOWS,
	type Opened
} from '../../testing/terminal-fixtures';

const { exits } = vi.hoisted(() => ({
	exits: { handler: null as null | ((event: { payload: { id: number; code: number | null } }) => void), stopped: 0 }
}));

vi.mock('@tauri-apps/api/event', () => ({
	listen: vi.fn(async (_name: string, handler: (typeof exits)['handler']) => {
		exits.handler = handler;
		return () => {
			exits.stopped += 1;
			exits.handler = null;
		};
	})
}));

vi.mock('$lib/api', () => ({
	TERMINAL_EXIT_EVENT: 'terminal-exit',
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
import { terminal } from './store.svelte';

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

/** Every terminal the "backend" opened, oldest first. */
let opened: Opened[] = [];
const bytes = (text: string) => new TextEncoder().encode(text);

/** The backend's PTYs: each open is kept with the channel its output goes down. */
function servePtys() {
	open.mockImplementation(async (ask, output) => {
		const id = opened.length + 1;
		opened.push({ id, ask, print: (text) => output(bytes(text)) });
		return id;
	});
}

const exit = (id: number, code: number | null = 0) => exits.handler?.({ payload: { id, code } });

beforeEach(() => {
	vi.clearAllMocks();
	localStorage.clear();
	terminal.clearState();
	emulators.length = 0;
	opened = [];
	exits.handler = null;
	exits.stopped = 0;

	terminal.use(makeRecorded);
	info.mockResolvedValue(TMUX);
	servePtys();
	sessionName.mockImplementation(async (path) => path.split(/[\\/]/).pop() || 'spagitty');
	sessions.mockResolvedValue(SESSIONS);
	windows.mockResolvedValue(WINDOWS);
	act.mockResolvedValue(undefined);
	links.mockResolvedValue({ commits: [], paths: [] });
});

describe('with a multiplexer', () => {
	it('attaches to the repository’s session, named for its folder, in its folder', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		expect(sessionName).toHaveBeenCalledWith('/repos/thing');
		expect(opened.map((one) => one.ask)).toEqual([
			{ session: 'thing', cwd: '/repos/thing', cols: 80, rows: 24 }
		]);
		expect(terminal.multiplexed).toBe(true);
		expect(terminal.current).toBe('thing');
		expect(terminal.has('thing')).toBe(true);
		expect(terminal.element('thing')).toBe(emulators[0].element);
	});

	it('reads which multiplexer it is once, however often the screen is shown', async () => {
		await terminal.enter('/repos/thing');
		await terminal.enter('/repos/thing');
		await settle();

		expect(info).toHaveBeenCalledTimes(1);
		expect(terminal.info).toEqual(TMUX);
		// And one client, not one for each visit.
		expect(opened).toHaveLength(1);
	});

	it('reads the sessions, and the windows of the one shown', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		expect(terminal.sessions).toEqual(SESSIONS);
		expect(windows).toHaveBeenCalledWith('thing');
		expect(terminal.windows).toEqual(WINDOWS);
	});

	it('has a tab for each session on the socket, and says which nobody here is attached to', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		expect(terminal.tabs).toEqual([
			{ key: 'thing', title: 'thing', detached: false },
			{ key: 'quiblo', title: 'quiblo', detached: true }
		]);
	});

	it('has a tab for the session being made before the multiplexer lists it', async () => {
		sessions.mockResolvedValue([]);

		await terminal.enter('/repos/fresh');
		await settle();

		expect(terminal.tabs).toEqual([{ key: 'fresh', title: 'fresh', detached: false }]);
	});

	it('attaches to another session when it is chosen, and keeps the first', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		terminal.select('quiblo', '/repos/thing');
		await settle();

		expect(opened.map((one) => one.ask.session)).toEqual(['thing', 'quiblo']);
		expect(terminal.current).toBe('quiblo');
		expect(terminal.has('thing')).toBe(true);
		expect(windows).toHaveBeenLastCalledWith('quiblo');

		// Back to the first: the client that is there, not another.
		terminal.select('thing', '/repos/thing');
		await settle();
		expect(opened).toHaveLength(2);
		expect(terminal.element('thing')).toBe(emulators[0].element);
	});

	it('asks the multiplexer for a window, a split and to be let go, on the session shown', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		await terminal.add('/repos/thing');
		await terminal.act({ kind: 'selectWindow', index: 2 });
		await terminal.act({ kind: 'splitRight' });
		await terminal.act({ kind: 'splitDown' });
		await terminal.act({ kind: 'detach' });

		expect(act.mock.calls).toEqual([
			['thing', { kind: 'newWindow', cwd: '/repos/thing' }],
			['thing', { kind: 'selectWindow', index: 2 }],
			['thing', { kind: 'splitRight' }],
			['thing', { kind: 'splitDown' }],
			['thing', { kind: 'detach' }]
		]);
		// `+` made a window, not another terminal.
		expect(opened).toHaveLength(1);
	});

	it('attaches again on `+` when the session’s client has gone, rather than asking a server that is not there (BUG-095)', async () => {
		await terminal.enter('/repos/thing');
		await settle();
		// The last shell in the session exited, and the client with it.
		exit(opened[0].id);
		await settle();
		expect(terminal.has('thing')).toBe(false);

		await terminal.add('/repos/thing');
		await settle();

		expect(act).not.toHaveBeenCalled();
		expect(opened.map((one) => one.ask.session)).toEqual(['thing', 'thing']);
		expect(terminal.current).toBe('thing');
		expect(terminal.has('thing')).toBe(true);
	});

	it('reads the windows again after asking for something', async () => {
		await terminal.enter('/repos/thing');
		await settle();
		windows.mockResolvedValue([...WINDOWS, { index: 4, name: 'shell', active: true }]);

		await terminal.add(null);

		expect(act).toHaveBeenCalledWith('thing', { kind: 'newWindow', cwd: '' });
		expect(terminal.windows).toHaveLength(4);
	});

	it('says what the multiplexer said when it refuses, and stops saying it when it does not', async () => {
		await terminal.enter('/repos/thing');
		await settle();
		act.mockRejectedValueOnce('no space for a new pane');

		await terminal.act({ kind: 'splitRight' });
		expect(terminal.failure).toBe('no space for a new pane');

		await terminal.act({ kind: 'splitDown' });
		expect(terminal.failure).toBeNull();
	});

	/** Detaching ends the client; the session carries on in the multiplexer. */
	it('lets go of a session whose client has ended, and leaves it to be attached to again', async () => {
		await terminal.enter('/repos/thing');
		await settle();
		const first = emulators[0];
		sessions.mockResolvedValue([{ name: 'thing', windows: 3, attached: false }]);

		exit(1);
		await settle();

		expect(first.disposed).toBe(true);
		expect(terminal.has('thing')).toBe(false);
		expect(terminal.element('thing')).toBeNull();
		// Still the one shown, and still a tab: it is detached, not gone.
		expect(terminal.current).toBe('thing');
		expect(terminal.tabs).toEqual([{ key: 'thing', title: 'thing', detached: true }]);
		expect(close).not.toHaveBeenCalled();

		terminal.attach('thing', '/repos/thing');
		await settle();
		expect(opened.map((one) => one.ask.session)).toEqual(['thing', 'thing']);
		expect(terminal.has('thing')).toBe(true);
	});

	it('never closes a session from here', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		terminal.close('thing');

		expect(close).not.toHaveBeenCalled();
		expect(terminal.has('thing')).toBe(true);
	});

	it('keeps the lists it has when they cannot be read', async () => {
		await terminal.enter('/repos/thing');
		await settle();
		sessions.mockRejectedValue('the server is busy');

		await terminal.refresh();

		expect(terminal.sessions).toEqual(SESSIONS);
		expect(terminal.windows).toEqual(WINDOWS);
		expect(terminal.failure).toBeNull();
	});
});

describe('follow repository', () => {
	it('is on until it is switched off, and stays off', () => {
		expect(terminal.follow).toBe(true);

		terminal.setFollow(false);
		expect(terminal.follow).toBe(false);
		expect(localStorage.getItem('spagitty.terminal.follow')).toBe('off');

		terminal.clearState();
		expect(terminal.follow).toBe(false);

		terminal.setFollow(true);
		expect(localStorage.getItem('spagitty.terminal.follow')).toBe('on');
	});

	it('switches to another repository’s session when that repository is opened, and makes it', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		await terminal.enter('/repos/quiblo');
		await settle();

		expect(terminal.current).toBe('quiblo');
		expect(opened.map((one) => [one.ask.session, one.ask.cwd])).toEqual([
			['thing', '/repos/thing'],
			['quiblo', '/repos/quiblo']
		]);
	});

	it('stays on the session shown when it is off', async () => {
		await terminal.enter('/repos/thing');
		await settle();
		terminal.setFollow(false);

		await terminal.enter('/repos/quiblo');
		await settle();

		expect(terminal.current).toBe('thing');
		expect(opened).toHaveLength(1);
	});

	it('still shows the open repository’s session the first time, when it is off', async () => {
		terminal.setFollow(false);

		await terminal.enter('/repos/thing');
		await settle();

		expect(terminal.current).toBe('thing');
	});

	it('has a session for no repository at all', async () => {
		await terminal.enter(null);
		await settle();

		expect(sessionName).toHaveBeenCalledWith('');
		expect(opened[0].ask).toMatchObject({ session: 'spagitty', cwd: '' });
	});
});

describe('with no multiplexer', () => {
	beforeEach(() => info.mockResolvedValue(PLAIN));

	it('opens a plain shell, named for its program, and asks the multiplexer nothing', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		expect(terminal.multiplexed).toBe(false);
		expect(opened.map((one) => one.ask)).toEqual([
			{ session: null, cwd: '/repos/thing', cols: 80, rows: 24 }
		]);
		expect(terminal.tabs).toEqual([{ key: 'shell-1', title: 'pwsh', detached: false }]);
		expect(terminal.current).toBe('shell-1');
		expect(sessionName).not.toHaveBeenCalled();
		expect(sessions).not.toHaveBeenCalled();
		expect(terminal.sessions).toEqual([]);
	});

	it('opens one shell on arrival and not another each time the screen is shown', async () => {
		await terminal.enter('/repos/thing');
		await terminal.enter('/repos/thing');
		await settle();

		expect(opened).toHaveLength(1);
	});

	it('`+` is another shell, and choosing a tab shows it', async () => {
		await terminal.enter('/repos/thing');
		await terminal.add('/repos/thing');
		await settle();

		expect(terminal.tabs.map((tab) => tab.title)).toEqual(['pwsh', 'pwsh 2']);
		expect(terminal.current).toBe('shell-2');
		expect(act).not.toHaveBeenCalled();

		terminal.select('shell-1', null);
		expect(terminal.current).toBe('shell-1');
		// A tab that is not there is not chosen.
		terminal.select('shell-9', null);
		expect(terminal.current).toBe('shell-1');
	});

	it('closing a tab ends its shell and shows the one beside it', async () => {
		await terminal.enter('/repos/thing');
		await terminal.add('/repos/thing');
		await settle();

		terminal.close('shell-2');

		expect(close).toHaveBeenCalledWith(2);
		expect(emulators[1].disposed).toBe(true);
		expect(terminal.current).toBe('shell-1');

		terminal.close('shell-1');
		expect(close).toHaveBeenCalledWith(1);
		expect(terminal.current).toBeNull();
		expect(terminal.tabs).toEqual([]);
	});

	it('a shell that exits takes its tab with it', async () => {
		await terminal.enter('/repos/thing');
		await terminal.add('/repos/thing');
		await settle();

		exit(2, 0);

		expect(terminal.tabs.map((tab) => tab.key)).toEqual(['shell-1']);
		expect(terminal.current).toBe('shell-1');
		// It has gone by itself: nothing is left to close.
		expect(close).not.toHaveBeenCalled();
	});

	it('a shell that exits behind the one shown leaves that one shown', async () => {
		await terminal.enter('/repos/thing');
		await terminal.add('/repos/thing');
		await settle();

		exit(1, 1);

		expect(terminal.current).toBe('shell-2');
		expect(terminal.tabs.map((tab) => tab.key)).toEqual(['shell-2']);
	});

	it('asks the multiplexer for nothing', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		await terminal.act({ kind: 'splitRight' });
		await terminal.followRepository('/repos/thing');
		await terminal.refresh();

		expect(act).not.toHaveBeenCalled();
		expect(sessionName).not.toHaveBeenCalled();
		expect(sessions).not.toHaveBeenCalled();
	});

	it('names a tab “shell” when the backend names no program', async () => {
		info.mockResolvedValue({ mux: null, shell: '' });

		await terminal.enter(null);
		await settle();

		expect(terminal.tabs[0].title).toBe('shell');
		expect(opened[0].ask.cwd).toBe('');
	});
});

describe('what the program prints and what is typed', () => {
	it('writes a frame’s output to the emulator once, as it was read', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		opened[0].print('one ');
		opened[0].print('two ');
		opened[0].print('thr☃e');
		expect(emulators[0].text()).toBe('');
		await settle();

		expect(emulators[0].text()).toBe('one two thr☃e');
		expect(emulators[0].writes).toBe(1);
	});

	/** The item: a million lines do not arrive as a million writes. */
	it('takes a flood in frames', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		for (let i = 0; i < 20_000; i++) opened[0].print('y\n');
		await settle();

		expect(emulators[0].text()).toHaveLength(40_000);
		expect(emulators[0].writes).toBeLessThan(10);
	});

	it('sends each terminal’s keystrokes to its own program', async () => {
		await terminal.enter('/repos/thing');
		terminal.select('quiblo', null);
		await settle();

		emulators[0].hooks.onData('ls\r');
		emulators[1].hooks.onData('\x03');

		expect(write.mock.calls).toEqual([
			[1, 'ls\r'],
			[2, '\x03']
		]);
	});

	it('keeps output arriving for a terminal that is not the one shown', async () => {
		await terminal.enter('/repos/thing');
		terminal.select('quiblo', null);
		await settle();

		opened[0].print('still building\r\n');
		await settle();

		expect(terminal.current).toBe('quiblo');
		expect(emulators[0].text()).toBe('still building\r\n');
	});

	/**
	 * A program's first output can ask where the cursor is, and ConPTY waits
	 * for the answer before it runs anything. The output comes down the
	 * channel, which can be ahead of the answer that carries the id.
	 */
	it('holds what the terminal sends before its program has an id, and sends it in order once it has', async () => {
		let answer: (id: number) => void = () => {};
		open.mockImplementation(() => new Promise<number>((resolve) => (answer = resolve)));

		await terminal.enter('/repos/thing');
		emulators[0].hooks.onData('\x1b[1;1R');
		emulators[0].hooks.onData('ls\r');
		expect(write).not.toHaveBeenCalled();

		answer(7);
		await settle();
		emulators[0].hooks.onData('now');
		expect(write.mock.calls).toEqual([
			[7, '\x1b[1;1Rls\r'],
			[7, 'now']
		]);
	});

	it('sends nothing held to a program whose tab was closed while it started', async () => {
		info.mockResolvedValue(PLAIN);
		let answer: (id: number) => void = () => {};
		open.mockImplementation(() => new Promise<number>((resolve) => (answer = resolve)));
		await terminal.enter('/repos/thing');
		emulators[0].hooks.onData('exit\r');

		terminal.close('shell-1');
		answer(4);
		await settle();

		expect(write).not.toHaveBeenCalled();
		expect(close).toHaveBeenCalledWith(4);
	});

	it('writes what was waiting before a terminal is let go', async () => {
		info.mockResolvedValue(PLAIN);
		await terminal.enter('/repos/thing');
		await settle();
		const shell = emulators[0];

		opened[0].print('logout\r\n');
		terminal.close('shell-1');

		expect(shell.text()).toBe('logout\r\n');
		expect(shell.disposed).toBe(true);
	});
});

describe('size', () => {
	it('tells the program the size its element holds, and not again until it changes', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		terminal.fit('thing');
		terminal.fit('thing');
		expect(resize.mock.calls).toEqual([[1, 132, 40]]);

		emulators[0].size = { cols: 100, rows: 30 };
		terminal.fit('thing');
		expect(resize).toHaveBeenLastCalledWith(1, 100, 30);
		expect(resize).toHaveBeenCalledTimes(2);
	});

	it('tells a program that was still starting the size it was fitted to meanwhile', async () => {
		let answer: (id: number) => void = () => {};
		open.mockImplementation(() => new Promise<number>((resolve) => (answer = resolve)));
		await terminal.enter('/repos/thing');

		terminal.fit('thing');
		expect(resize).not.toHaveBeenCalled();

		answer(3);
		await settle();
		expect(resize.mock.calls).toEqual([[3, 132, 40]]);
	});

	it('fits and focuses nothing for a terminal that is not there', () => {
		terminal.fit(null);
		terminal.fit('nobody');
		terminal.focus(null);
		terminal.focus('nobody');

		expect(resize).not.toHaveBeenCalled();
	});

	it('gives the focus to the terminal asked for', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		terminal.focus('thing');

		expect(emulators[0].focused).toBe(1);
	});
});

describe('when a terminal cannot be had', () => {
	it('says why the program could not be started, and keeps no tab for it', async () => {
		info.mockResolvedValue(PLAIN);
		open.mockRejectedValue('fish could not be started: program not found');

		await terminal.enter('/repos/thing');
		await settle();

		expect(terminal.failure).toBe('fish could not be started: program not found');
		expect(terminal.tabs).toEqual([]);
		expect(terminal.current).toBeNull();
		expect(emulators[0].disposed).toBe(true);
	});

	it('leaves a session that could not be attached to as the one shown, detached', async () => {
		open.mockRejectedValue('tmux could not be started');

		await terminal.enter('/repos/thing');
		await settle();

		expect(terminal.failure).toBe('tmux could not be started');
		expect(terminal.current).toBe('thing');
		expect(terminal.has('thing')).toBe(false);
	});

	it('forgets the failure when a terminal is opened after it', async () => {
		info.mockResolvedValue(PLAIN);
		open.mockRejectedValueOnce('no');
		await terminal.enter('/repos/thing');
		await settle();
		expect(terminal.failure).toBe('no');

		servePtys();
		await terminal.add('/repos/thing');
		await settle();

		expect(terminal.failure).toBeNull();
		expect(terminal.current).toBe('shell-2');
	});

	it('ends a program that arrives after its tab was closed', async () => {
		info.mockResolvedValue(PLAIN);
		let answer: (id: number) => void = () => {};
		open.mockImplementation(() => new Promise<number>((resolve) => (answer = resolve)));
		await terminal.enter('/repos/thing');

		terminal.close('shell-1');
		expect(close).not.toHaveBeenCalled();
		answer(5);
		await settle();

		expect(close).toHaveBeenCalledWith(5);
		expect(terminal.tabs).toEqual([]);
	});

	it('says so when it cannot be told what runs a terminal', async () => {
		info.mockRejectedValue('settings could not be read');

		await terminal.enter('/repos/thing');
		await settle();

		expect(terminal.failure).toBe('settings could not be read');
		expect(opened).toEqual([]);
	});

	it('makes no terminal before it has been given an emulator', async () => {
		terminal.clearState();

		await terminal.enter('/repos/thing');
		await settle();

		expect(opened).toEqual([]);
		expect(emulators).toEqual([]);
	});

	it('takes an exit for a terminal it does not have as nothing', async () => {
		await terminal.enter('/repos/thing');
		await settle();

		exit(99);

		expect(terminal.has('thing')).toBe(true);
	});
});

describe('links', () => {
	const sha = 'a3f9c1e' + '0'.repeat(33);
	const linksOn = (line: string) => emulators[0].hooks.links(line);

	beforeEach(async () => {
		await terminal.enter('/repos/thing');
		await settle();
		links.mockImplementation(async (words, paths) => ({
			commits: words.filter((word) => word === 'a3f9c1e').map((word) => [word, sha] as [string, string]),
			paths: paths.filter((path) => path === 'src/a.ts')
		}));
	});

	it('underlines only what the repository says is real', async () => {
		const found = await linksOn('a3f9c1e decade1 src/a.ts:3 src/gone.ts:9');

		expect(links).toHaveBeenCalledWith(['a3f9c1e', 'decade1'], ['src/a.ts', 'src/gone.ts']);
		expect(found).toEqual([
			{ kind: 'commit', start: 0, end: 7, text: 'a3f9c1e', commit: sha },
			{ kind: 'file', start: 16, end: 26, text: 'src/a.ts:3', path: 'src/a.ts', line: 3 }
		]);
	});

	/** A screen of `git log` is asked about once. */
	it('asks about a word once, whether it was a commit or not', async () => {
		await linksOn('a3f9c1e decade1');
		const again = await linksOn('a3f9c1e decade1 src/a.ts:1');

		expect(links.mock.calls).toEqual([
			[['a3f9c1e', 'decade1'], []],
			[[], ['src/a.ts']]
		]);
		expect(again.map((link) => link.text)).toEqual(['a3f9c1e', 'src/a.ts:1']);

		await linksOn('a3f9c1e decade1 src/a.ts:1');
		expect(links).toHaveBeenCalledTimes(2);
	});

	it('asks nothing for a line with nothing on it that could be one', async () => {
		expect(await linksOn('Compiling spagitty v3.0.0')).toEqual([]);
		expect(links).not.toHaveBeenCalled();
	});

	it('underlines nothing it could not check, and asks again the next time', async () => {
		links.mockRejectedValueOnce('no repository');

		expect(await linksOn('a3f9c1e')).toEqual([]);
		expect((await linksOn('a3f9c1e')).map((link) => link.text)).toEqual(['a3f9c1e']);
		expect(links).toHaveBeenCalledTimes(2);
	});

	it('forgets what it was told when another repository is opened', async () => {
		await linksOn('a3f9c1e');
		links.mockResolvedValue({ commits: [], paths: [] });

		terminal.forgetLinks();

		expect(await linksOn('a3f9c1e')).toEqual([]);
		expect(links).toHaveBeenCalledTimes(2);
	});

	it('opens a link with what the screen gave it to open one with', async () => {
		const openLink = vi.fn();
		terminal.onOpen(openLink);
		const [link] = await linksOn('a3f9c1e');

		emulators[0].hooks.open(link);

		expect(openLink).toHaveBeenCalledWith(link);
	});
});

describe('Ctrl+`', () => {
	it('goes to Terminal from anywhere, and back to where you were', () => {
		expect(terminal.toggle('/changes')).toBe('/terminal');
		expect(terminal.from).toBe('/changes');
		expect(terminal.toggle('/terminal')).toBe('/changes');

		expect(terminal.toggle('/files?path=a.rs&line=3')).toBe('/terminal');
		expect(terminal.toggle('/terminal?x=1')).toBe('/files?path=a.rs&line=3');
	});

	it('goes back to the Graph when Terminal was the first screen', () => {
		expect(terminal.toggle('/terminal')).toBe('/');
	});
});

describe('starting again', () => {
	it('lets every terminal go and stops listening', async () => {
		await terminal.enter('/repos/thing');
		terminal.select('quiblo', null);
		await settle();

		terminal.clearState();

		expect(emulators.every((one) => one.disposed)).toBe(true);
		expect(exits.stopped).toBe(1);
		expect(terminal.info).toBeNull();
		expect(terminal.tabs).toEqual([]);
		expect(terminal.current).toBeNull();
		expect(terminal.sessions).toEqual([]);
		expect(terminal.windows).toEqual([]);
		expect(terminal.from).toBe('/');
	});
});
