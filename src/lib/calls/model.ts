// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Calls' pure rules (FEAT-160): a collection's tree, a URL's query as rows,
 * what each tab counts, which variable a name finds, and how a response is
 * summed up. Nothing here reaches the backend; the store does.
 */

import { namesIn, isBuiltin } from './vars';
import type { Call, Collection, Environment, Folder, Item, Pair, Shared, Variable } from './types';

export const isFolder = (item: Item): item is Folder => 'folder' in item;

/** Every call in `items`, folders opened, in order. */
export function callsIn(items: Item[]): Call[] {
	return items.flatMap((item) => (isFolder(item) ? callsIn(item.items) : [item]));
}

export function findCall(items: Item[], id: string): Call | null {
	return callsIn(items).find((call) => call.id === id) ?? null;
}

/** `items` with the call `id` put through `change`. */
export function mapCall(items: Item[], id: string, change: (call: Call) => Call): Item[] {
	return items.map((item) =>
		isFolder(item) ? { ...item, items: mapCall(item.items, id, change) } : item.id === id ? change(item) : item
	);
}

export function removeCall(items: Item[], id: string): Item[] {
	return items.flatMap<Item>((item) =>
		isFolder(item) ? [{ ...item, items: removeCall(item.items, id) }] : item.id === id ? [] : [item]
	);
}

/** `items` with `call` added at the end of `folder`, or at the top level. */
export function addCall(items: Item[], call: Call, folder: string | null = null): Item[] {
	if (folder === null) return [...items, call];
	let placed = false;
	const out = items.map((item) => {
		if (!placed && isFolder(item) && item.folder === folder) {
			placed = true;
			return { ...item, items: [...item.items, call] };
		}
		return item;
	});
	return placed ? out : [...items, { folder, items: [call] }];
}

/** The folder a call is in, by name, or null at the top level. */
export function folderOf(items: Item[], id: string): string | null {
	for (const item of items) {
		if (isFolder(item) && callsIn(item.items).some((call) => call.id === id)) return item.folder;
	}
	return null;
}

/**
 * The calls whose method, name or URL contain `query`, in their folders;
 * a folder with none left is dropped. An empty query keeps everything.
 */
export function filterItems(items: Item[], query: string): Item[] {
	const needle = query.trim().toLowerCase();
	if (!needle) return items;
	return items.flatMap<Item>((item) => {
		if (isFolder(item)) {
			const kept = filterItems(item.items, query);
			return kept.length ? [{ ...item, items: kept }] : [];
		}
		const hay = `${methodOf(item)} ${item.name} ${item.url}`.toLowerCase();
		return hay.includes(needle) ? [item] : [];
	});
}

const ALPHABET = '0123456789abcdefghjkmnpqrstvwxyz';

/** An id as `calls::model::new_id` makes one: eight characters of time, then four of chance. */
export function newId(now = Date.now(), random = Math.random): string {
	let out = 'c_';
	for (let shift = 7; shift >= 0; shift--) out += ALPHABET[Math.floor(now / 2 ** (shift * 5)) % 32];
	for (let i = 0; i < 4; i++) out += ALPHABET[Math.floor(random() * 32)];
	return out;
}

export function blankCall(name = 'New call'): Call {
	return { id: newId(), name, kind: 'rest', method: 'GET', url: '' };
}

/** A copy of `call` under a new id, named as a copy. */
export function duplicateOf(call: Call): Call {
	return { ...plain(call), id: newId(), name: `${call.name} copy` };
}

/** Plain data from a value that may be a Svelte proxy. */
export function plain<T>(value: T): T {
	return JSON.parse(JSON.stringify(value)) as T;
}

export const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS'] as const;

export function methodOf(call: Call): string {
	return call.kind === 'soap' ? 'SOAP' : (call.method ?? 'GET').toUpperCase();
}

/** The colour token a method is drawn in (the handoff's map onto `app.css`). */
export function methodColour(method: string): string {
	switch (method.toUpperCase()) {
		case 'GET':
			return 'var(--ok)';
		case 'POST':
			return 'var(--lane-5)';
		case 'PUT':
			return 'var(--warn)';
		case 'PATCH':
			return 'var(--lane-4)';
		case 'DELETE':
			return 'var(--danger)';
		case 'SOAP':
			return 'var(--agent)';
		default:
			return 'var(--muted)';
	}
}

/** An environment's colour as a token. */
export function envColour(colour: Environment['color'] | undefined): string {
	switch (colour) {
		case 'ok':
			return 'var(--ok)';
		case 'warn':
			return 'var(--warn)';
		case 'danger':
			return 'var(--danger)';
		case 'blue':
			return 'var(--lane-5)';
		case 'teal':
			return 'var(--lane-2)';
		default:
			return 'var(--muted)';
	}
}

/** The URL cut at its query: what is before `?`, the query, and a `#` part. */
function cut(url: string): { base: string; query: string | null; hash: string } {
	const hashAt = url.indexOf('#');
	const hash = hashAt === -1 ? '' : url.slice(hashAt);
	const rest = hashAt === -1 ? url : url.slice(0, hashAt);
	const at = rest.indexOf('?');
	return at === -1 ? { base: rest, query: null, hash } : { base: rest.slice(0, at), query: rest.slice(at + 1), hash };
}

/** The URL's query as rows, as written: nothing is decoded, so `{{x}}` stays. */
export function paramsOf(url: string): Pair[] {
	const { query } = cut(url);
	if (!query) return [];
	return query
		.split('&')
		.filter((part) => part !== '')
		.map((part) => {
			const at = part.indexOf('=');
			return at === -1 ? [part, ''] : [part.slice(0, at), part.slice(at + 1)];
		});
}

/** `url` with its query written from `params`; a row switched off is left out. */
export function withParams(url: string, params: Pair[]): string {
	const { base, hash } = cut(url);
	const on = params.filter((pair) => pair[2] !== false && (pair[0] !== '' || pair[1] !== ''));
	if (!on.length) return base + hash;
	return `${base}?${on.map(([name, value]) => (value === '' ? name : `${name}=${value}`)).join('&')}${hash}`;
}

/** What each request tab holds, for the count beside its name; 0 shows none. */
export function tabCounts(call: Call): { params: number; headers: number; body: number; auth: number; keep: number } {
	const body = call.body;
	return {
		params: paramsOf(call.url).length,
		headers: (call.headers ?? []).filter((pair) => pair[2] !== false).length,
		body: !body || body.type === 'none' ? 0 : body.type === 'form' || body.type === 'multipart' ? (body.fields ?? []).length || 1 : 1,
		auth: call.auth && call.auth.type !== 'none' ? 1 : 0,
		keep: (call.keep ?? []).length
	};
}

/** A variable as the screen finds it: where it is defined, and what it holds. */
export interface Found {
	name: string;
	/** `environment`, `shared` or `collection`; a built-in is `builtin`. */
	from: 'environment' | 'shared' | 'collection' | 'builtin';
	variable: Variable | null;
}

/**
 * Where `name` is defined, in resolution order (proposal §5.2): the active
 * environment, then shared, then the collection's own. Null when nothing
 * defines it.
 */
export function lookup(
	name: string,
	environment: Environment | null,
	shared: Shared | null,
	collection: Collection | null
): Found | null {
	if (isBuiltin(name)) return { name, from: 'builtin', variable: null };
	const pick = (vars: Variable[] | undefined) => vars?.find((variable) => variable.name === name) ?? null;
	const inEnv = pick(environment?.vars);
	if (inEnv) return { name, from: 'environment', variable: inEnv };
	const inShared = pick(shared?.vars);
	if (inShared) return { name, from: 'shared', variable: inShared };
	const inCollection = pick(collection?.vars);
	if (inCollection) return { name, from: 'collection', variable: inCollection };
	return null;
}

/** Every text in `call` that variables can be written in. */
export function textsOf(call: Call): string[] {
	const out = [call.url];
	for (const pair of call.headers ?? []) if (pair[2] !== false) out.push(pair[0], pair[1]);
	const body = call.body;
	if (body && body.type !== 'none') {
		if (body.text) out.push(body.text);
		for (const field of body.fields ?? []) if (field.on !== false) out.push(field.name, field.value);
	}
	const auth = call.auth;
	if (auth && auth.type !== 'none') out.push(auth.token ?? '', auth.user ?? '', auth.password ?? '', auth.key ?? '', auth.value ?? '');
	if (call.kind === 'soap') out.push(call.envelope ?? '');
	return out;
}

/** The names `call` uses that nothing defines. */
export function missingIn(
	call: Call,
	environment: Environment | null,
	shared: Shared | null,
	collection: Collection | null
): string[] {
	const names = [...new Set(textsOf(call).flatMap(namesIn))];
	return names.filter((name) => !lookup(name, environment, shared, collection));
}

/** `1.4 KB`, as the response bar shows a size. */
export function formatSize(bytes: number): string {
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
	return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function formatMillis(millis: number): string {
	return millis < 1000 ? `${millis} ms` : `${(millis / 1000).toFixed(2)} s`;
}

/** The tone of a status: 2xx is ok, 3xx is plain, 4xx a warning, 5xx danger. */
export function statusTone(status: number): 'ok' | 'plain' | 'warn' | 'danger' {
	if (status >= 500) return 'danger';
	if (status >= 400) return 'warn';
	if (status >= 300) return 'plain';
	return 'ok';
}

export interface Cookie {
	name: string;
	value: string;
	/** The attributes after it, as sent: `Path=/; HttpOnly`. */
	attributes: string;
}

/** The cookies a response sets, from its `Set-Cookie` headers. */
export function cookiesOf(headers: [string, string][]): Cookie[] {
	return headers
		.filter(([name]) => name.toLowerCase() === 'set-cookie')
		.map(([, value]) => {
			const [first, ...rest] = value.split(';');
			const at = first.indexOf('=');
			return {
				name: (at === -1 ? first : first.slice(0, at)).trim(),
				value: at === -1 ? '' : first.slice(at + 1).trim(),
				attributes: rest.map((part) => part.trim()).join('; ')
			};
		});
}

/** Keys too plain to name a variable alone: `user.id` is kept as `userId`. */
const PLAIN_KEYS = new Set(['id', 'uuid', 'key', 'name', 'value', 'code', 'type', 'url', 'href']);

/**
 * The name a value is suggested to be kept as (FEAT-161, proposal §6.2):
 * the last key in its path, with `accessToken` becoming `token` and a plain
 * key such as `id` taking its parent's name, so `$.data.user.id` is
 * `userId`. An XPath gives its last element's or attribute's local name.
 */
export function suggestName(path: string): string {
	const keys: string[] = [];
	if (path.startsWith('$')) {
		for (const match of path.matchAll(/\.([A-Za-z_$][\w$]*)|\[(['"])((?:\\.|(?!\2).)*)\2\]/g)) {
			keys.push(match[1] ?? match[3].replace(/\\(.)/g, '$1'));
		}
	} else {
		for (const step of path.split('/')) {
			const name = step.replace(/^@/, '').replace(/\[\d+\]$/, '');
			if (name) keys.push(name.slice(name.indexOf(':') + 1));
		}
	}
	const camel = (word: string) =>
		word
			.replace(/[^A-Za-z0-9]+(.)?/g, (_, next: string | undefined) => (next ? next.toUpperCase() : ''))
			.replace(/^[A-Z]/, (first) => first.toLowerCase());
	const last = camel(keys[keys.length - 1] ?? '');
	if (!last) return 'value';
	if (/^accessToken$|^idToken$|^bearerToken$|^authToken$/i.test(last)) return 'token';
	const parent = camel(keys[keys.length - 2] ?? '');
	if (PLAIN_KEYS.has(last.toLowerCase()) && parent && parent !== 'data') {
		return parent + last[0].toUpperCase() + last.slice(1);
	}
	return /^\d/.test(last) ? `v${last}` : last;
}
