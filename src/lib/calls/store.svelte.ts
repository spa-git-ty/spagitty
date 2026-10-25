// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The Calls screen's state (FEAT-160).
 *
 * # What is edited, and when it is written
 *
 * The collections on screen are the files under the calls folder, read once
 * when the repository is opened. An edit changes the one in memory at once
 * and writes its file a moment later, so a word typed is one write; a file
 * that could not be read is listed with its error and never edited. The
 * person's own state (the active environment, the open calls, what is
 * collapsed, the filter) is written to application data the same way.
 *
 * # A send
 *
 * *Send* hands the call as it is on screen to the backend, which resolves
 * its variables and refuses at once, with nothing sent, when one has no
 * value. The answer arrives as a `calls-done` event, and an event stream
 * arrives piece by piece as `calls-chunk` before it. Each open call has its
 * own send and its own last response; switching calls does not stop one.
 *
 * An environment with *Ask before sending* holds the send until the person
 * says yes in the bar that names it.
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { describe, notice } from '$lib/ui/notice.svelte';
import * as api from './api';
import {
	addCall,
	blankCall,
	duplicateOf,
	findCall,
	folderOf,
	mapCall,
	missingIn,
	plain,
	removeCall
} from './model';
import type {
	Call,
	CallsSettings,
	Chunk,
	Collection,
	Done,
	Environment,
	Environments,
	HistoryEntry,
	KeepRule,
	Loaded,
	Opened,
	PersonState,
	Private,
	ResponseView,
	Shared,
	Shown,
	Stored
} from './types';

/** How long after an edit its file is written. */
export const SAVE_DELAY = 400;

/** What came back for an open call. */
export interface Outcome {
	sendId: string;
	response: ResponseView | null;
	error: string | null;
	cancelled: boolean;
	sent: Shown | null;
	/** An event stream so far, while it arrives. */
	streamed: string;
	/** Shown as received rather than formatted. */
	raw: boolean;
}

const EMPTY_STATE: PersonState = { active: null, open: [], current: null, collapsed: [], filter: '' };

let loaded = $state(false);
let loadError = $state<string | null>(null);
let folder = $state('.spagitty/calls');
let collections = $state<Stored<Collection>[]>([]);
let environments = $state<Stored<Environment>[]>([]);
let shared = $state<Shared>({ v: 1, vars: [] });
let settings = $state<CallsSettings | null>(null);
let person = $state<PersonState>({ ...EMPTY_STATE });
let today = $state(0);
/** In flight, by call key: the send's id. */
let sending = $state<Record<string, string>>({});
let outcomes = $state<Record<string, Outcome>>({});
/** A send waiting for the person to say yes: the call's key. */
let asking = $state<string | null>(null);
/** Why the current call cannot be sent, said under the URL. */
let refusals = $state<Record<string, string>>({});
let history = $state<HistoryEntry[] | null>(null);
/** The active environment's kept values, and which secrets have one (FEAT-161). */
let privateValues = $state<Private>({ kept: [], secrets: [] });
/** Why the keychain could not be read. */
let privateError = $state<string | null>(null);

let repoPath: string | null = null;
let privateSeq = 0;
const envTimers = new Map<string, ReturnType<typeof setTimeout>>();
let unlisten: UnlistenFn[] = [];
let listening: Promise<void> | null = null;
const saveTimers = new Map<string, ReturnType<typeof setTimeout>>();
let stateTimer: ReturnType<typeof setTimeout> | null = null;

export const keyOf = (opened: Opened) => `${opened.stem}/${opened.id}`;

function sendId(): string {
	return `s_${Date.now().toString(36)}${Math.floor(Math.random() * 1e9).toString(36)}`;
}

function storedOf(stem: string): Stored<Collection> | null {
	return collections.find((stored) => stored.stem === stem) ?? null;
}

function activeEnvironment(): Environment | null {
	return environments.find((stored) => stored.stem === person.active)?.value ?? null;
}

function saveState(): void {
	if (stateTimer) clearTimeout(stateTimer);
	stateTimer = setTimeout(() => {
		stateTimer = null;
		api.saveState(plain(person)).catch((cause) => notice.failed('Could not keep the open calls', cause));
	}, SAVE_DELAY);
}

async function writeCollection(stem: string): Promise<void> {
	saveTimers.delete(stem);
	const stored = storedOf(stem);
	if (!stored?.value) return;
	try {
		await api.saveCollection(plain(stored.value), stem, null);
	} catch (cause) {
		notice.failed(`Could not save ${stored.file}`, cause);
	}
}

function saveSoon(stem: string): void {
	const timer = saveTimers.get(stem);
	if (timer) clearTimeout(timer);
	saveTimers.set(
		stem,
		setTimeout(() => void writeCollection(stem), SAVE_DELAY)
	);
}

/** Write every edit still waiting: before another repository is read. */
async function flush(): Promise<void> {
	const waiting = [...saveTimers.keys()];
	for (const stem of waiting) {
		clearTimeout(saveTimers.get(stem));
		await writeCollection(stem);
	}
	for (const [stem, timer] of [...envTimers]) {
		clearTimeout(timer);
		envTimers.delete(stem);
		const value = environments.find((stored) => stored.stem === stem)?.value;
		if (value) await api.saveEnvironment(plain(value), stem).catch(() => {});
	}
	if (stateTimer) {
		clearTimeout(stateTimer);
		stateTimer = null;
		await api.saveState(plain(person)).catch(() => {});
	}
}

function changeCollection(stem: string, change: (collection: Collection) => Collection): void {
	const index = collections.findIndex((stored) => stored.stem === stem);
	const value = collections[index]?.value;
	if (!value) return;
	collections[index] = { ...collections[index], value: change(value) };
	saveSoon(stem);
}

function takeEnvironments(read: Environments): void {
	environments = read.environments;
	shared = read.shared;
}

/** Read the active environment's private side again. */
async function refreshPrivate(): Promise<void> {
	const ticket = ++privateSeq;
	try {
		const read = await api.privateOf(person.active);
		if (ticket !== privateSeq) return;
		privateValues = read;
		privateError = null;
	} catch (cause) {
		if (ticket !== privateSeq) return;
		privateValues = { kept: [], secrets: [] };
		privateError = describe(cause);
	}
}

/** A send kept values: the file may name new ones, and the values changed. */
async function afterKept(): Promise<void> {
	try {
		takeEnvironments(await api.environments());
	} catch {
		// What is on screen stays; the next load reads it.
	}
	await refreshPrivate();
}

function writeEnvironmentSoon(stem: string): void {
	const timer = envTimers.get(stem);
	if (timer) clearTimeout(timer);
	envTimers.set(
		stem,
		setTimeout(async () => {
			envTimers.delete(stem);
			const value = environments.find((stored) => stored.stem === stem)?.value;
			if (!value) return;
			try {
				await api.saveEnvironment(plain(value), stem);
			} catch (cause) {
				notice.failed(`Could not save the environment ${value.name}`, cause);
			}
		}, SAVE_DELAY)
	);
}

function onDone(done: Done): void {
	const key = Object.keys(sending).find((candidate) => sending[candidate] === done.sendId);
	if (!key) return;
	const { [key]: _, ...rest } = sending;
	sending = rest;
	const before = outcomes[key];
	if (done.cancelled) {
		outcomes[key] = {
			sendId: done.sendId,
			response: null,
			error: null,
			cancelled: true,
			sent: null,
			streamed: before?.sendId === done.sendId ? before.streamed : '',
			raw: false
		};
		return;
	}
	today += 1;
	if (done.keptError) notice.failed('Could not keep a value from the response', done.keptError);
	if (done.kept?.length) void afterKept();
	outcomes[key] = {
		sendId: done.sendId,
		response: done.response,
		error: done.error,
		cancelled: false,
		sent: done.sent,
		streamed: '',
		raw: before?.raw ?? false
	};
	if (history) void calls.loadHistory();
}

function onChunk(chunk: Chunk): void {
	const key = Object.keys(sending).find((candidate) => sending[candidate] === chunk.sendId);
	if (!key) return;
	const before = outcomes[key];
	outcomes[key] = {
		sendId: chunk.sendId,
		response: null,
		error: null,
		cancelled: false,
		sent: null,
		streamed: (before?.sendId === chunk.sendId ? before.streamed : '') + chunk.text,
		raw: before?.raw ?? false
	};
}

async function listenOnce(): Promise<void> {
	listening ??= (async () => {
		unlisten = [
			await listen<Done>(api.DONE_EVENT, (event) => onDone(event.payload)),
			await listen<Chunk>(api.CHUNK_EVENT, (event) => onChunk(event.payload))
		];
	})();
	await listening;
}

/** A name for a new collection: the repository's folder name, or `calls`. */
function collectionName(): string {
	const name = repoPath?.split(/[\\/]/).filter(Boolean).pop() ?? '';
	const clean = name.toLowerCase().replace(/[^a-z0-9._-]+/g, '-');
	return clean || 'calls';
}

export const calls = {
	get loaded(): boolean {
		return loaded;
	},
	get error(): string | null {
		return loadError;
	},
	get folder(): string {
		return folder;
	},
	get collections(): Stored<Collection>[] {
		return collections;
	},
	get environments(): Stored<Environment>[] {
		return environments;
	},
	get shared(): Shared {
		return shared;
	},
	get settings(): CallsSettings | null {
		return settings;
	},
	get person(): PersonState {
		return person;
	},
	get today(): number {
		return today;
	},
	get asking(): string | null {
		return asking;
	},
	get history(): HistoryEntry[] | null {
		return history;
	},
	get private(): Private {
		return privateValues;
	},
	get privateError(): string | null {
		return privateError;
	},

	/** The environment calls are sent in, or null for none. */
	get environment(): Environment | null {
		return activeEnvironment();
	},

	/** The open calls that still exist, with what each is. */
	get tabs(): { opened: Opened; call: Call }[] {
		return person.open.flatMap((opened) => {
			const call = findCall(storedOf(opened.stem)?.value?.items ?? [], opened.id);
			return call ? [{ opened, call }] : [];
		});
	},

	get current(): { opened: Opened; call: Call; collection: Collection } | null {
		const opened = person.current;
		if (!opened) return null;
		const collection = storedOf(opened.stem)?.value;
		const call = collection ? findCall(collection.items, opened.id) : null;
		return call && collection ? { opened, call, collection } : null;
	},

	sendingFor(opened: Opened): boolean {
		return keyOf(opened) in sending;
	},
	outcomeFor(opened: Opened): Outcome | null {
		return outcomes[keyOf(opened)] ?? null;
	},
	refusalFor(opened: Opened): string | null {
		return refusals[keyOf(opened)] ?? null;
	},

	/** Read the repository's calls. Edits waiting for the last one are written first. */
	async load(path: string): Promise<void> {
		if (path !== repoPath) {
			await flush();
			sending = {};
			outcomes = {};
			refusals = {};
			asking = null;
			history = null;
		}
		repoPath = path;
		try {
			await listenOnce();
			const read: Loaded = await api.load();
			if (repoPath !== path) return;
			folder = read.folder;
			collections = read.collections;
			environments = read.environments;
			shared = read.shared;
			settings = read.settings;
			today = read.today;
			person = { ...EMPTY_STATE, ...read.state };
			if (person.active && !environments.some((stored) => stored.stem === person.active)) person.active = null;
			if (!person.active && environments[0]?.value) person.active = environments[0].stem;
			loadError = read.sharedError ? `shared.json: ${read.sharedError}` : null;
			await refreshPrivate();
		} catch (cause) {
			loadError = describe(cause);
		} finally {
			loaded = true;
		}
	},

	open(opened: Opened): void {
		if (!person.open.some((other) => keyOf(other) === keyOf(opened))) person.open = [...person.open, opened];
		person.current = opened;
		saveState();
	},

	/** Close an open call; the one beside it becomes current. */
	close(opened: Opened): void {
		const at = person.open.findIndex((other) => keyOf(other) === keyOf(opened));
		if (at === -1) return;
		const open = person.open.filter((_, index) => index !== at);
		if (person.current && keyOf(person.current) === keyOf(opened)) {
			person.current = open[Math.min(at, open.length - 1)] ?? null;
		}
		person.open = open;
		if (asking === keyOf(opened)) asking = null;
		saveState();
	},

	setActive(stem: string | null): void {
		person.active = stem;
		asking = null;
		saveState();
		void refreshPrivate();
	},

	setFilter(filter: string): void {
		person.filter = filter;
		saveState();
	},

	toggleCollapsed(id: string): void {
		person.collapsed = person.collapsed.includes(id)
			? person.collapsed.filter((other) => other !== id)
			: [...person.collapsed, id];
		saveState();
	},

	/** Change the call `opened` names; its file is written a moment later. */
	edit(opened: Opened, change: (call: Call) => Call): void {
		changeCollection(opened.stem, (collection) => ({
			...collection,
			items: mapCall(collection.items, opened.id, change)
		}));
		const key = keyOf(opened);
		if (refusals[key]) {
			const { [key]: _, ...rest } = refusals;
			refusals = rest;
		}
	},

	/**
	 * A new call, opened: in `stem`'s folder `folderName`, or in the first
	 * collection that can be edited, or in a new collection named after the
	 * repository.
	 */
	async create(stem: string | null = null, folderName: string | null = null): Promise<void> {
		const call = blankCall();
		let target = stem ?? collections.find((stored) => stored.value)?.stem ?? null;
		if (!target) {
			const collection: Collection = { v: 1, name: collectionName(), items: [call] };
			try {
				target = await api.saveCollection(collection, null, null);
			} catch (cause) {
				notice.failed('Could not make a collection', cause);
				return;
			}
			collections = [
				...collections,
				{ stem: target, file: `${folder}/${target}.json`, value: collection, error: null }
			];
		} else {
			changeCollection(target, (collection) => ({
				...collection,
				items: addCall(collection.items, call, folderName)
			}));
		}
		this.open({ stem: target, id: call.id });
	},

	duplicate(opened: Opened): void {
		const collection = storedOf(opened.stem)?.value;
		const call = collection ? findCall(collection.items, opened.id) : null;
		if (!collection || !call) return;
		const copy = duplicateOf(call);
		changeCollection(opened.stem, (value) => ({
			...value,
			items: addCall(value.items, copy, folderOf(value.items, opened.id))
		}));
		this.open({ stem: opened.stem, id: copy.id });
	},

	remove(opened: Opened): void {
		this.close(opened);
		changeCollection(opened.stem, (collection) => ({
			...collection,
			items: removeCall(collection.items, opened.id)
		}));
	},

	/** The names `opened`'s call uses that nothing defines in the active environment. */
	missing(opened: Opened): string[] {
		const collection = storedOf(opened.stem)?.value ?? null;
		const call = collection ? findCall(collection.items, opened.id) : null;
		return call ? missingIn(call, activeEnvironment(), shared, collection) : [];
	},

	/**
	 * Send the current call, or ask first when its environment says to.
	 * `confirmed` is the person's yes in that bar.
	 */
	async send(confirmed = false): Promise<void> {
		const current = this.current;
		if (!current) return;
		const key = keyOf(current.opened);
		if (key in sending) return;
		const env = activeEnvironment();
		const missing = this.missing(current.opened);
		if (missing.length) {
			const where = env ? `in ${env.name}` : 'with no environment chosen';
			refusals[key] = `Not sent: ${missing.map((name) => `{{${name}}}`).join(', ')} ${missing.length === 1 ? 'has' : 'have'} no value ${where}.`;
			return;
		}
		if (env?.askBeforeSending && !confirmed) {
			asking = key;
			return;
		}
		asking = null;
		const id = sendId();
		sending = { ...sending, [key]: id };
		try {
			await api.send({
				sendId: id,
				stem: current.opened.stem,
				call: plain(current.call),
				environment: person.active
			});
		} catch (cause) {
			const { [key]: _, ...rest } = sending;
			sending = rest;
			refusals[key] = describe(cause);
		}
	},

	/** Stop waiting for the current call's send. */
	async cancel(opened: Opened): Promise<void> {
		const id = sending[keyOf(opened)];
		if (!id) return;
		try {
			await api.cancel(id);
		} catch (cause) {
			notice.failed('Could not cancel the call', cause);
		}
	},

	dismissAsk(): void {
		asking = null;
	},

	/** Switch one response between formatted and as received. */
	toggleRaw(opened: Opened): void {
		const outcome = outcomes[keyOf(opened)];
		if (outcome) outcomes[keyOf(opened)] = { ...outcome, raw: !outcome.raw };
	},

	async saveBody(opened: Opened, path: string): Promise<void> {
		const outcome = outcomes[keyOf(opened)];
		if (!outcome) return;
		try {
			await api.saveBody(outcome.sendId, path);
			notice.ok('Response saved');
		} catch (cause) {
			notice.failed('Could not save the response', cause);
		}
	},

	async loadHistory(): Promise<void> {
		try {
			history = await api.history();
		} catch (cause) {
			notice.failed('Could not read the history', cause);
		}
	},

	closeHistory(): void {
		history = null;
	},

	async clearHistory(): Promise<void> {
		try {
			await api.clearHistory();
			history = [];
			today = 0;
		} catch (cause) {
			notice.failed('Could not clear the history', cause);
		}
	},

	// --- Keeping values (FEAT-161) ---

	/**
	 * Keep `value`, found at `path` in the response to `opened`'s call, as
	 * `name` in the active environment. The rule goes on the call, replacing
	 * one for the same name or the same path; the value goes to the keychain.
	 */
	async keep(
		opened: Opened,
		ask: { path: string; value: string; name: string; every: boolean; ns?: Record<string, string> }
	): Promise<boolean> {
		const collection = storedOf(opened.stem)?.value;
		const call = collection ? findCall(collection.items, opened.id) : null;
		const name = ask.name.trim();
		if (!call || !name) return false;
		const rule: KeepRule = { var: name, from: ask.path, ...(ask.every ? { every: true } : {}) };
		if (ask.ns && Object.keys(ask.ns).length) rule.ns = ask.ns;
		try {
			takeEnvironments(
				await api.keep({
					environment: person.active,
					var: name,
					value: ask.value,
					callId: call.id,
					call: call.name,
					method: call.kind === 'soap' ? 'SOAP' : (call.method ?? 'GET'),
					from: ask.path
				})
			);
		} catch (cause) {
			notice.failed(`Could not keep ${name}`, cause);
			return false;
		}
		this.edit(opened, (current) => ({
			...current,
			keep: [...(current.keep ?? []).filter((other) => other.var !== name && other.from !== ask.path), rule]
		}));
		await refreshPrivate();
		return true;
	},

	/** Stop keeping `name`: its rule leaves the call, its value the keychain. */
	async forget(opened: Opened, name: string): Promise<void> {
		this.edit(opened, (current) => ({
			...current,
			keep: (current.keep ?? []).filter((rule) => rule.var !== name)
		}));
		try {
			takeEnvironments(await api.forgetKept(person.active, name));
		} catch (cause) {
			notice.failed(`Could not forget ${name}`, cause);
		}
		await refreshPrivate();
	},

	/**
	 * Forget a kept value in an environment (`null` for shared), from
	 * Environments. The calls' rules stay: the next send that finds it keeps
	 * it again.
	 */
	async forgetIn(environment: string | null, name: string): Promise<void> {
		try {
			takeEnvironments(await api.forgetKept(environment, name));
		} catch (cause) {
			notice.failed(`Could not forget ${name}`, cause);
		}
		if (environment === person.active) await refreshPrivate();
	},

	// --- Environments (FEAT-161) ---

	/** Change an environment in place; its file is written a moment later. */
	editEnvironment(stem: string, change: (environment: Environment) => Environment): void {
		const index = environments.findIndex((stored) => stored.stem === stem);
		const value = environments[index]?.value;
		if (!value) return;
		environments[index] = { ...environments[index], value: change(value) };
		writeEnvironmentSoon(stem);
	},

	/** Rename an environment: a new file, and its keychain values moved. */
	async renameEnvironment(stem: string, name: string): Promise<boolean> {
		const value = environments.find((stored) => stored.stem === stem)?.value;
		if (!value || name === stem) return false;
		const timer = envTimers.get(stem);
		if (timer) clearTimeout(timer);
		envTimers.delete(stem);
		try {
			takeEnvironments(await api.saveEnvironment({ ...plain(value), name }, stem));
		} catch (cause) {
			notice.failed(`Could not rename ${value.name}`, cause);
			return false;
		}
		if (person.active === stem) {
			person.active = name;
			saveState();
		}
		return true;
	},

	/** A new environment, asking before sending when it is production or red. */
	async createEnvironment(name: string, color: Environment['color'], copyFrom: string | null = null): Promise<boolean> {
		const from = copyFrom ? environments.find((stored) => stored.stem === copyFrom)?.value : null;
		const environment: Environment = from
			? { ...plain(from), name }
			: {
					v: 1,
					name,
					color,
					askBeforeSending: name.toLowerCase() === 'production' || color === 'danger',
					vars: []
				};
		try {
			takeEnvironments(await api.saveEnvironment(environment, null, copyFrom));
			return true;
		} catch (cause) {
			notice.failed(`Could not make the environment ${name}`, cause);
			return false;
		}
	},

	async deleteEnvironment(stem: string): Promise<void> {
		try {
			takeEnvironments(await api.deleteEnvironment(stem));
		} catch (cause) {
			notice.failed(`Could not delete ${stem}`, cause);
			return;
		}
		if (person.active === stem) this.setActive(environments.find((stored) => stored.value)?.stem ?? null);
	},

	/** Change the shared variables; written at once. */
	async editShared(change: (shared: Shared) => Shared): Promise<void> {
		shared = change(shared);
		try {
			takeEnvironments(await api.saveShared(plain(shared)));
		} catch (cause) {
			notice.failed('Could not save the shared variables', cause);
		}
	},

	/** Set a secret's value in an environment (`null` for shared). */
	async setSecret(environment: string | null, name: string, value: string | null): Promise<void> {
		try {
			await api.setPrivate(environment, name, value);
		} catch (cause) {
			notice.failed(`Could not keep ${name} in the keychain`, cause);
		}
		if (environment === person.active) await refreshPrivate();
	},

	reveal(environment: string | null, name: string): Promise<string | null> {
		return api.reveal(environment, name).catch(() => null);
	},

	privateOf(environment: string | null): Promise<Private> {
		return api.privateOf(environment);
	},

	/** For tests: forget everything and stop listening. */
	reset(): void {
		for (const timer of envTimers.values()) clearTimeout(timer);
		envTimers.clear();
		privateValues = { kept: [], secrets: [] };
		privateError = null;
		for (const timer of saveTimers.values()) clearTimeout(timer);
		saveTimers.clear();
		if (stateTimer) clearTimeout(stateTimer);
		stateTimer = null;
		for (const stop of unlisten) stop();
		unlisten = [];
		listening = null;
		repoPath = null;
		loaded = false;
		loadError = null;
		collections = [];
		environments = [];
		shared = { v: 1, vars: [] };
		settings = null;
		person = { ...EMPTY_STATE };
		today = 0;
		sending = {};
		outcomes = {};
		refusals = {};
		asking = null;
		history = null;
	}
};
