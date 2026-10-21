// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Terminal screen's state (FEAT-130).
 *
 * # Two shapes, one screen
 *
 * With a multiplexer, a terminal is its client attached to a **session** on
 * Spagitty's own socket — one session for each repository, named for its
 * folder. The tabs are the sessions, the panel lists them with their windows,
 * and leaving a terminal detaches: the session carries on. Closing one ends
 * the session, after the screen has asked (FEAT-157).
 *
 * With none, a terminal is a plain shell, the tabs are the shells that are
 * open, and there is no panel. Nothing on the screen says a multiplexer is
 * missing, because nothing is.
 *
 * # Terminals outlive the screen
 *
 * An emulator and its element are kept here, not in a component. Leaving the
 * Terminal screen takes the element out of the page and nothing else; the
 * program keeps running, its output keeps being written, and coming back
 * puts the same element back with everything that was printed while away.
 *
 * # Links are checked, never guessed
 *
 * A hex word or a `path:line` in the output is underlined only after the
 * repository has said it is a commit, or a file at HEAD. What it said is kept,
 * so a screen of `git log` is asked about once.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as api from '../api';
import type {
	TerminalAsk,
	TerminalInfo,
	TerminalSession,
	TerminalStyle,
	TerminalWindow
} from '../types';
import { Batcher } from './batch';
import type { Emulator, MakeEmulator } from './emulator';
import { linksIn, pathsIn, wordsIn, type Link } from './links';

const FOLLOW_KEY = 'spagitty.terminal.follow';
/** How often the sessions and windows are read again while the screen is shown. */
export const REFRESH_MS = 2000;

/** One terminal: a program in a PTY, and the emulator that shows it. */
interface Terminal {
	/** A session's name, or a plain tab's own key. */
	key: string;
	/** Its id in the backend, once it has been opened. */
	id: number | null;
	/** The session it is attached to. Null for a plain shell. */
	session: string | null;
	title: string;
	emulator: Emulator;
	batcher: Batcher;
	/** The size it was last fitted to, sent once it has an id. */
	size: { cols: number; rows: number } | null;
	/**
	 * What the terminal sent before its program had an id, in order. Not only
	 * keys: a program's first output can ask where the cursor is, and ConPTY
	 * waits for the answer before it runs anything. An answer dropped here is
	 * a terminal that never starts.
	 */
	early: string[];
}

/** A tab in the header. */
export interface Tab {
	key: string;
	title: string;
	/** A session nobody is attached to here: pressing it attaches. */
	detached: boolean;
}

let info = $state<TerminalInfo | null>(null);
let sessions = $state<TerminalSession[]>([]);
let windows = $state<TerminalWindow[]>([]);
let follow = $state(readFollow());
let current = $state<string | null>(null);
let failure = $state<string | null>(null);
/** Bumped when a terminal is made or goes, which is what redraws the tabs. */
let changed = $state(0);
/** The screen that was open before Terminal, for the way back. */
let from = $state('/');

const terminals = new Map<string, Terminal>();
let make: MakeEmulator | null = null;
let plainCount = 0;
let unlisten: UnlistenFn | null = null;
let listening = false;

const known = { commits: new Map<string, string>(), paths: new Set<string>() };
const asked = new Set<string>();
/** What the link checker opens a commit and a file with. Set by the screen. */
let opener: (link: Link) => void = () => {};
/** The theme's colours for the multiplexer, read when asked. Set by the screen. */
let styler: (() => TerminalStyle) | null = null;

/** Give the multiplexer's bars the theme's colours (FEAT-157). */
function style(): void {
	if (!styler || info?.mux == null) return;
	void api.terminalStyle(styler()).catch(() => {
		// The colours stay as they were. The next change sends them again.
	});
}

function readFollow(): boolean {
	try {
		return localStorage.getItem(FOLLOW_KEY) !== 'off';
	} catch {
		return true;
	}
}

function remove(key: string): void {
	const gone = terminals.get(key);
	if (!gone) return;
	gone.batcher.flush();
	gone.emulator.dispose();
	terminals.delete(key);
	changed += 1;
}

/** A terminal's program has exited: a shell that was closed, or a client that detached. */
function exited(id: number): void {
	const gone = [...terminals.values()].find((terminal) => terminal.id === id);
	if (!gone) return;
	const wasCurrent = gone.key === current;
	remove(gone.key);
	if (gone.session === null && wasCurrent) {
		// A plain tab that closed: the one beside it, or none.
		current = [...terminals.keys()].at(-1) ?? null;
	}
	if (gone.session !== null) void terminal.refresh();
}

async function listenForExits(): Promise<void> {
	if (listening) return;
	listening = true;
	try {
		unlisten = await listen<{ id: number; code: number | null }>(api.TERMINAL_EXIT_EVENT, (event) =>
			exited(event.payload.id)
		);
	} catch {
		// Not in the application: nothing will exit.
	}
}

/** Check a line's candidates with the repository, once each. */
async function links(line: string): Promise<Link[]> {
	const words = wordsIn(line)
		.map((candidate) => candidate.text)
		.filter((word) => !asked.has(`w:${word}`));
	const paths = pathsIn(line)
		.map((candidate) => candidate.path)
		.filter((path) => !asked.has(`p:${path}`));
	if (words.length > 0 || paths.length > 0) {
		for (const word of words) asked.add(`w:${word}`);
		for (const path of paths) asked.add(`p:${path}`);
		try {
			const answer = await api.terminalLinks(words, paths);
			for (const [word, commit] of answer.commits) known.commits.set(word, commit);
			for (const path of answer.paths) known.paths.add(path);
		} catch {
			// Not checked is not a link. They may be asked about again.
			for (const word of words) asked.delete(`w:${word}`);
			for (const path of paths) asked.delete(`p:${path}`);
		}
	}
	return linksIn(line, known);
}

/** Start a terminal: an emulator now, and its program as soon as the backend answers. */
function open(key: string, session: string | null, title: string, cwd: string): Terminal | null {
	if (!make) return null;
	const created: Terminal = {
		key,
		id: null,
		session,
		title,
		size: null,
		early: [],
		batcher: new Batcher((bytes) => created.emulator.write(bytes)),
		emulator: make({
			onData: (data) => {
				if (created.id !== null) void api.terminalWrite(created.id, data);
				else created.early.push(data);
			},
			links,
			open: (link) => opener(link)
		})
	};
	terminals.set(key, created);
	changed += 1;
	failure = null;

	const size = created.size ?? { cols: 80, rows: 24 };
	api.terminalOpen({ session, cwd, ...size }, (bytes) => created.batcher.push(bytes)).then(
		(id) => {
			// Closed while it was opening: its program is not wanted.
			if (terminals.get(key) !== created) {
				void api.terminalClose(id);
				return;
			}
			created.id = id;
			if (created.early.length > 0) void api.terminalWrite(id, created.early.splice(0).join(''));
			// Fitted while it was opening: tell the program the size it really has.
			if (created.size && (created.size.cols !== size.cols || created.size.rows !== size.rows)) {
				void api.terminalResize(id, created.size.cols, created.size.rows);
			}
			if (session !== null) {
				// A client that has just started may have started the server
				// with it, from the person's own config: the colours go in after.
				style();
				void terminal.refresh();
			}
		},
		(error) => {
			if (terminals.get(key) !== created) return;
			remove(key);
			if (current === key && session === null) current = null;
			failure = String(error);
		}
	);
	return created;
}

export const terminal = {
	get info(): TerminalInfo | null {
		return info;
	},
	/** True when the shell runs inside a multiplexer. */
	get multiplexed(): boolean {
		return info?.mux != null;
	},
	get sessions(): TerminalSession[] {
		return sessions;
	},
	get windows(): TerminalWindow[] {
		return windows;
	},
	get follow(): boolean {
		return follow;
	},
	get current(): string | null {
		return current;
	},
	get failure(): string | null {
		return failure;
	},
	get from(): string {
		return from;
	},

	/**
	 * The header's tabs. With a multiplexer, the sessions on Spagitty's socket
	 * and any being opened; without, the shells that are open.
	 */
	get tabs(): Tab[] {
		void changed;
		if (!this.multiplexed) {
			return [...terminals.values()].map((entry) => ({ key: entry.key, title: entry.title, detached: false }));
		}
		const names = new Set(sessions.map((session) => session.name));
		for (const entry of terminals.values()) names.add(entry.key);
		if (current) names.add(current);
		return [...names].map((name) => ({ key: name, title: name, detached: !terminals.has(name) }));
	},

	/** Whether a terminal is open here for `key`: a session attached to, or a shell. */
	has(key: string | null): boolean {
		void changed;
		return key !== null && terminals.has(key);
	},

	/** The element the terminal for `key` draws in, to be put in the screen. */
	element(key: string | null): HTMLElement | null {
		void changed;
		return key === null ? null : (terminals.get(key)?.emulator.element ?? null);
	},

	/** The emulator to make terminals with: xterm.js, or a recording one in tests. */
	use(next: MakeEmulator): void {
		make = next;
	},

	/** What the multiplexer's colours are read with: the theme's tokens. */
	styleWith(next: () => TerminalStyle): void {
		styler = next;
	},

	/** The theme changed: every terminal takes its colours again, and the multiplexer. */
	restyle(): void {
		for (const entry of terminals.values()) entry.emulator.retheme();
		if (terminals.size > 0) style();
	},

	/** What opening a link does: the Graph for a commit, Files for a place in a file. */
	onOpen(next: (link: Link) => void): void {
		opener = next;
	},

	/**
	 * The screen is shown, with `repository` open (or none). Reads what the
	 * multiplexer is, once, and shows that repository's terminal.
	 */
	async enter(repository: string | null): Promise<void> {
		void listenForExits();
		if (!info) {
			try {
				info = await api.terminalInfo();
			} catch (error) {
				failure = String(error);
				return;
			}
		}
		if (this.multiplexed) {
			if (current === null || follow) await this.followRepository(repository);
			await this.refresh();
		} else if (terminals.size === 0) {
			this.newTab(repository);
		}
	},

	/** Attach to `repository`'s session, making it if it is not there. */
	async followRepository(repository: string | null): Promise<void> {
		if (!this.multiplexed) return;
		const name = await api.terminalSessionName(repository ?? '');
		this.attach(name, repository ?? '');
	},

	/** Show a session, attaching to it if no client of ours is. */
	attach(name: string, cwd: string): void {
		current = name;
		if (!terminals.has(name)) open(name, name, name, cwd);
		void this.refresh();
	},

	/** A new plain shell, in `cwd`. */
	newTab(cwd: string | null): void {
		plainCount += 1;
		const key = `shell-${plainCount}`;
		const program = (info?.shell ?? '').split(/[\\/]/).pop()?.replace(/\.exe$/i, '') || 'shell';
		const opened = open(key, null, plainCount === 1 ? program : `${program} ${plainCount}`, cwd ?? '');
		if (opened) current = key;
	},

	/** `+`: a new window in the session, or a new shell. */
	async add(cwd: string | null): Promise<void> {
		if (!this.multiplexed) {
			this.newTab(cwd);
			return;
		}
		// Nothing shown, as after the last session was closed: this
		// repository's session, made again.
		if (!current) {
			await this.followRepository(cwd);
			return;
		}
		// No client of ours on it: its last shell exited, or the multiplexer
		// could not start. There may be no session to add a window to, and
		// asking says *no server running* (BUG-095). Attaching makes it again.
		if (!terminals.has(current)) {
			this.attach(current, cwd ?? '');
			return;
		}
		await this.act({ kind: 'newWindow', cwd: cwd ?? '' });
	},

	select(key: string, cwd: string | null): void {
		if (this.multiplexed) this.attach(key, cwd ?? '');
		else if (terminals.has(key)) current = key;
	},

	/**
	 * Close a terminal: a plain shell, or a session with every window in it
	 * (FEAT-157). The screen asks first; this does not.
	 */
	async close(key: string): Promise<void> {
		const gone = terminals.get(key);
		if (!this.multiplexed || gone?.session === null) {
			if (!gone) return;
			if (gone.id !== null) void api.terminalClose(gone.id);
			remove(key);
			if (current === key) current = [...terminals.keys()].at(-1) ?? null;
			return;
		}
		try {
			await api.terminalAct(key, { kind: 'close' });
			failure = null;
		} catch (error) {
			failure = String(error);
			return;
		}
		// Ending the session ends our client too; it is let go of here rather
		// than when its exit arrives, so the tab goes at once.
		if (gone) {
			if (gone.id !== null) void api.terminalClose(gone.id);
			remove(key);
		}
		sessions = sessions.filter((session) => session.name !== key);
		if (current === key) {
			current = null;
			current = this.tabs.find((tab) => tab.key !== key)?.key ?? null;
		}
		await this.refresh();
	},

	/** Ask the multiplexer for something on the session shown. */
	async act(action: TerminalAsk): Promise<void> {
		if (!this.multiplexed || !current) return;
		try {
			await api.terminalAct(current, action);
			failure = null;
		} catch (error) {
			failure = String(error);
		}
		await this.refresh();
	},

	/** Read the sessions, and the windows of the one shown. */
	async refresh(): Promise<void> {
		if (!this.multiplexed) return;
		try {
			sessions = await api.terminalSessions();
			windows = current ? await api.terminalWindows(current) : [];
		} catch {
			// The lists are a moment old. The next reading mends them.
		}
	},

	setFollow(on: boolean): void {
		follow = on;
		try {
			localStorage.setItem(FOLLOW_KEY, on ? 'on' : 'off');
		} catch {
			// Only for this visit.
		}
	},

	/** Size the terminal for `key` to its element, and tell its program. */
	fit(key: string | null): void {
		const entry = key === null ? undefined : terminals.get(key);
		if (!entry) return;
		const size = entry.emulator.fit();
		if (entry.size && entry.size.cols === size.cols && entry.size.rows === size.rows) return;
		entry.size = size;
		if (entry.id !== null) void api.terminalResize(entry.id, size.cols, size.rows);
	},

	focus(key: string | null): void {
		if (key !== null) terminals.get(key)?.emulator.focus();
	},

	/**
	 * `Ctrl+``: to Terminal from anywhere, and back from it to where you were.
	 * Returns where to go.
	 */
	toggle(here: string): string {
		if (here === '/terminal' || here.startsWith('/terminal?')) return from;
		from = here;
		return '/terminal';
	},

	/** Forget which links have been checked: another repository is open. */
	forgetLinks(): void {
		known.commits.clear();
		known.paths.clear();
		asked.clear();
	},

	clearState(): void {
		for (const key of [...terminals.keys()]) remove(key);
		unlisten?.();
		unlisten = null;
		listening = false;
		info = null;
		sessions = [];
		windows = [];
		follow = readFollow();
		current = null;
		failure = null;
		from = '/';
		plainCount = 0;
		make = null;
		opener = () => {};
		styler = null;
		this.forgetLinks();
	}
};
