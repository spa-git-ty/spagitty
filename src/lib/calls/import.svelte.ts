// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * Calls › Import's state (FEAT-162, proposal §8).
 *
 * A scan runs in the backend on a thread of its own and says how far it
 * has got; what it found is kept there too, so *Bring in* names endpoints
 * by id and the backend builds them with the choices made here. Files from
 * outside and a pasted cURL are added to what the scan found.
 *
 * *New* and *Changed* endpoints are ticked when they arrive; ones already
 * in a collection are not (§8.1).
 */

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { describe, notice } from '$lib/ui/notice.svelte';
import * as api from './api';
import { calls } from './store.svelte';
import type { BroughtIn, Endpoint, Found, ScanDone, ScanProgress, SourceKind } from './types';

export type Filter = 'all' | 'new' | 'changed' | 'in';

/** Sources whose endpoints are declared by code rather than held in a file. */
export const CODE_KINDS: SourceKind[] = ['spring', 'jaxRs', 'nest', 'jaxWs', 'springWs', 'wsdl', 'openApi'];

export interface BringChoices {
	/** Into an existing collection, or a new one. */
	as: 'into' | 'new';
	into: string | null;
	name: string;
	folders: 'perGroup' | 'byPath';
	bodies: boolean;
	pathVars: boolean;
	serverAddress: boolean;
}

let found = $state<Found | null>(null);
let scanning = $state<{ id: string; files: number } | null>(null);
let error = $state<string | null>(null);
let selected = $state<string[]>([]);
let filter = $state<Filter>('all');
let source = $state<string | null>(null);
let bringing = $state(false);
let choices = $state<BringChoices>({
	as: 'into',
	into: null,
	name: '',
	folders: 'perGroup',
	bodies: true,
	pathVars: true,
	serverAddress: true
});
/** The Calls card's question was answered for this repository. */
let dismissed = $state(false);

let repoPath: string | null = null;
let listening: Promise<void> | null = null;
let unlisten: UnlistenFn[] = [];
/** Scanned once on arrival when Settings says *Always*. */
let autoScanned = new Set<string>();

const wanted = (endpoint: Endpoint) => endpoint.status.kind === 'new' || endpoint.status.kind === 'changed';

function take(next: Found | null, from: string | null = null): void {
	const before = new Set(found?.endpoints.map((endpoint) => endpoint.id) ?? []);
	found = next;
	const ids = new Set(next?.endpoints.map((endpoint) => endpoint.id) ?? []);
	// What is still there stays as chosen; what arrived is ticked by the rule.
	const kept = selected.filter((id) => ids.has(id));
	const arrived = (next?.endpoints ?? [])
		.filter((endpoint) => !before.has(endpoint.id) || (from !== null && endpoint.source === from))
		.filter(wanted)
		.map((endpoint) => endpoint.id);
	selected = [...new Set([...kept, ...arrived])];
	if (choices.into === null || !calls.collections.some((stored) => stored.stem === choices.into)) {
		const counts = new Map<string, number>();
		for (const endpoint of next?.endpoints ?? []) {
			if (endpoint.status.kind !== 'new') counts.set(endpoint.status.collection, (counts.get(endpoint.status.collection) ?? 0) + 1);
		}
		const best = [...counts].sort((a, b) => b[1] - a[1])[0]?.[0];
		const first = calls.collections.find((stored) => stored.value)?.stem ?? null;
		choices.into = best ?? first;
		if (!choices.into) choices.as = 'new';
	}
	if (!choices.name) {
		const name = repoPath?.split(/[\\/]/).filter(Boolean).pop() ?? 'calls';
		choices.name = name.toLowerCase().replace(/[^a-z0-9._-]+/g, '-') || 'calls';
	}
}

async function listenOnce(): Promise<void> {
	listening ??= (async () => {
		unlisten = [
			await listen<ScanProgress>(api.SCAN_EVENT, (event) => {
				if (scanning && event.payload.scanId === scanning.id) scanning = { ...scanning, files: event.payload.files };
			}),
			await listen<ScanDone>(api.SCAN_DONE_EVENT, (event) => {
				const done = event.payload;
				if (!scanning || done.scanId !== scanning.id) return;
				scanning = null;
				if (done.cancelled) return;
				if (done.error) {
					error = done.error;
					return;
				}
				error = null;
				take(done.found);
			})
		];
	})();
	await listening;
}

export const importer = {
	get found(): Found | null {
		return found;
	},
	get scanning(): { id: string; files: number } | null {
		return scanning;
	},
	get error(): string | null {
		return error;
	},
	get selected(): string[] {
		return selected;
	},
	get filter(): Filter {
		return filter;
	},
	get source(): string | null {
		return source;
	},
	get choices(): BringChoices {
		return choices;
	},
	get bringing(): boolean {
		return bringing;
	},
	get dismissed(): boolean {
		return dismissed;
	},

	/** The endpoints the filters leave. */
	get visible(): Endpoint[] {
		return (found?.endpoints ?? []).filter(
			(endpoint) => (source === null || endpoint.source === source) && (filter === 'all' || endpoint.status.kind === filter)
		);
	},

	/** Endpoints code declares that no collection has: what the Calls card counts. */
	get notInACollection(): Endpoint[] {
		const code = new Set(
			(found?.sources ?? []).filter((s) => CODE_KINDS.includes(s.kind)).map((s) => s.id)
		);
		return (found?.endpoints ?? []).filter((endpoint) => code.has(endpoint.source) && endpoint.status.kind === 'new');
	},

	counts(): Record<Filter, number> {
		const all = (found?.endpoints ?? []).filter((endpoint) => source === null || endpoint.source === source);
		return {
			all: all.length,
			new: all.filter((e) => e.status.kind === 'new').length,
			changed: all.filter((e) => e.status.kind === 'changed').length,
			in: all.filter((e) => e.status.kind === 'in').length
		};
	},

	/** What this repository's last scan found; a scan when Settings says *Always*. */
	async load(path: string): Promise<void> {
		if (path !== repoPath) {
			repoPath = path;
			found = null;
			selected = [];
			source = null;
			filter = 'all';
			error = null;
			dismissed = false;
			choices = { ...choices, into: null, name: '' };
		}
		await listenOnce();
		try {
			take(await api.found());
		} catch (cause) {
			error = describe(cause);
		}
		if (!found && calls.settings?.scanOnOpen === 'always' && !autoScanned.has(path)) {
			autoScanned.add(path);
			await this.scan();
		}
	},

	async scan(): Promise<void> {
		if (scanning) return;
		const id = `scan_${Date.now().toString(36)}`;
		scanning = { id, files: 0 };
		error = null;
		dismissed = true;
		try {
			await listenOnce();
			await api.scan(id);
		} catch (cause) {
			scanning = null;
			error = describe(cause);
		}
	},

	async cancel(): Promise<void> {
		if (!scanning) return;
		const id = scanning.id;
		scanning = null;
		await api.cancelScan(id).catch(() => {});
	},

	dismiss(): void {
		dismissed = true;
	},

	async importFile(path: string): Promise<boolean> {
		try {
			const next = await api.importFile(path);
			const added = next.sources.at(-1)?.id ?? null;
			take(next, added);
			source = added;
			return true;
		} catch (cause) {
			notice.failed('Could not read that file', cause);
			return false;
		}
	},

	/** A WSDL from a URL the person typed (FEAT-163). */
	async importWsdlUrl(url: string): Promise<boolean> {
		try {
			const next = await api.importWsdlUrl(url);
			const added = next.sources.at(-1)?.id ?? null;
			take(next, added);
			source = added;
			return true;
		} catch (cause) {
			notice.failed('Could not read that WSDL', cause);
			return false;
		}
	},

	async importText(text: string): Promise<boolean> {
		try {
			const next = await api.importText(text);
			const added = next.sources.at(-1)?.id ?? null;
			take(next, added);
			source = added;
			return true;
		} catch (cause) {
			notice.failed('Could not read that cURL command', cause);
			return false;
		}
	},

	setFilter(next: Filter): void {
		filter = next;
	},

	setSource(next: string | null): void {
		source = next;
	},

	toggle(id: string): void {
		selected = selected.includes(id) ? selected.filter((other) => other !== id) : [...selected, id];
	},

	/** Tick or untick every endpoint in `ids` together. */
	toggleAll(ids: string[]): void {
		const all = ids.every((id) => selected.includes(id));
		selected = all ? selected.filter((id) => !ids.includes(id)) : [...new Set([...selected, ...ids])];
	},

	choose(change: Partial<BringChoices>): void {
		choices = { ...choices, ...change };
	},

	/** Bring the ticked endpoints in, then read the collections again. */
	async bringIn(): Promise<BroughtIn | null> {
		if (!selected.length || bringing) return null;
		bringing = true;
		try {
			const done = await api.bringIn({
				ids: selected,
				into: choices.as === 'into' ? choices.into : null,
				name: choices.as === 'new' ? choices.name : null,
				choices: { folders: choices.folders, bodies: choices.bodies, pathVars: choices.pathVars },
				serverAddress: choices.serverAddress
			});
			const parts = [
				done.brought.added ? `${done.brought.added} added` : '',
				done.brought.changed ? `${done.brought.changed} updated` : '',
				done.environments ? `${done.environments} environment${done.environments === 1 ? '' : 's'} made` : ''
			].filter(Boolean);
			notice.ok(`Brought into ${done.stem}: ${parts.join(', ') || 'nothing new'}`);
			if (repoPath) await calls.load(repoPath);
			selected = [];
			take(await api.found());
			choices = { ...choices, as: 'into', into: done.stem };
			return done;
		} catch (cause) {
			notice.failed('Could not bring them in', cause);
			return null;
		} finally {
			bringing = false;
		}
	},

	/** For tests. */
	reset(): void {
		for (const stop of unlisten) stop();
		unlisten = [];
		listening = null;
		repoPath = null;
		autoScanned = new Set();
		found = null;
		scanning = null;
		error = null;
		selected = [];
		filter = 'all';
		source = null;
		bringing = false;
		dismissed = false;
		choices = { as: 'into', into: null, name: '', folders: 'perGroup', bodies: true, pathVars: true, serverAddress: true };
	}
};
