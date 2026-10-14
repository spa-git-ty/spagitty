// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * The builder owns its draft; run records remain on disk. Epochs keep a slow
 * read or a finished command from showing another repository's hooks.
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { dialog } from '$lib/ui/dialog.svelte';
import * as api from './builder-api';
import { makeBlock, steps, type Action, type Block, type Builder, type Definition, type Run, type Stage, type Tried } from './builder-types';

let data = $state<Builder | null>(null);
let draft = $state<Definition | null>(null);
let saved = '';
let path: string | null = null;
let epoch = 0;
let reading = 0;
let picking = 0;
let matching = 0;
let loading = $state(false);
let saving = $state(false);
let running = $state(false);
let imported = $state(false);
let selected = $state<string | null>(null);
let current = $state<Run | null>(null);
let selectedStep = $state<string | null>(null);
let log = $state('');
let matches = $state<string[]>([]);
let tried = $state<Tried | null>(null);
let errors = $state({ load: '', save: '', run: '', records: '', log: '', matches: '' });
let unlisten: UnlistenFn | null = null;
const snapshot = () => draft ? $state.snapshot(draft) : null;

export const builder = {
	get data() { return data; }, get draft() { return draft; }, get loading() { return loading; },
	get saving() { return saving; }, get running() { return running; }, get errors() { return errors; },
	get imported() { return imported; }, get dirty() { return draft !== null && JSON.stringify(draft) !== saved; },
	get readonly() { return data !== null && data.info.manager !== 'git' && !imported; },
	get block() { return draft?.blocks.find((block) => block.id === selected) ?? null; },
	get current() { return current; }, get selectedStep() { return selectedStep; }, get log() { return log; },
	get matches() { return matches; }, get tried() { return tried; },

	async load(repo: string | null) {
		if (repo && repo === path && data) {
			const ticket = epoch;
			if (!unlisten) {
				const release = await listen('hooks-changed', () => { if (ticket === epoch) void this.refreshRuns(); });
				if (ticket !== epoch) { release(); return; }
				unlisten = release;
			}
			void this.refreshRuns(); return;
		}
		this.clear(); path = repo; if (!repo) return;
		const ticket = epoch; loading = true;
		try {
			const release = await listen('hooks-changed', () => { if (ticket === epoch) void this.refreshRuns(); });
			if (ticket !== epoch) { release(); return; } unlisten = release;
			const result = await api.load(); if (ticket !== epoch) return;
			data = result; draft = structuredClone(result.definition); saved = JSON.stringify(draft);
			selected = draft.blocks[0]?.id ?? null;
			if (result.runs[0]) await this.openRun(result.runs[0].id);
			void this.checkMatches();
		} catch (cause) { if (ticket === epoch) errors.load = String(cause); }
		finally { if (ticket === epoch) loading = false; }
	},
	detach() { unlisten?.(); unlisten = null; },
	clear() {
		epoch++; reading++; picking++; matching++; unlisten?.(); unlisten = null; path = null;
		data = null; draft = null; saved = ''; selected = null; current = null; selectedStep = null;
		log = ''; matches = []; tried = null; imported = false; loading = false; saving = false; running = false;
		errors = { load: '', save: '', run: '', records: '', log: '', matches: '' };
	},
	select(id: string) { selected = id; tried = null; void this.checkMatches(); },
	add(kind: Action['kind'], stage: Stage = 'pre-commit') {
		if (!draft || this.readonly) return;
		const block = makeBlock(kind, stage); draft.blocks.push(block); selected = block.id; void this.checkMatches();
	},
	set(block: Block) {
		if (!draft || this.readonly) return;
		draft.blocks = draft.blocks.map((old) => old.id === block.id ? block : old); tried = null; void this.checkMatches();
	},
	remove(id: string) {
		if (!draft || this.readonly) return; draft.blocks = draft.blocks.filter((block) => block.id !== id);
		selected = draft.blocks[0]?.id ?? null; void this.checkMatches();
	},
	move(id: string, delta: number) {
		if (!draft || this.readonly) return;
		const blocks = draft.blocks; const from = blocks.findIndex((block) => block.id === id);
		if (from < 0) return;
		const siblings = blocks.map((block, at) => ({ block, at })).filter(({ block }) => block.stage === blocks[from].stage);
		const target = siblings[siblings.findIndex(({ at }) => at === from) + delta]?.at;
		if (target === undefined) return;
		[blocks[from], blocks[target]] = [blocks[target], blocks[from]];
	},
	share(shared: boolean) { if (draft && !this.readonly) draft.shared = shared; },
	toggleStage(stage: Stage) {
		if (!draft || this.readonly) return;
		draft.activeStages = draft.activeStages.includes(stage) ? draft.activeStages.filter((value) => value !== stage) : [...draft.activeStages, stage];
	},
	async checkMatches() {
		const block = this.block; const ticket = ++matching; const owner = epoch;
		if (!block || !('paths' in block.action)) { matches = []; return; }
		try { const result = await api.matches([...block.action.paths]); if (owner === epoch && ticket === matching) { matches = result; errors.matches = ''; } }
		catch (cause) { if (owner === epoch && ticket === matching) errors.matches = String(cause); }
	},
	async save(install = true) {
		if (!draft || saving || this.readonly) return false;
		const definition = snapshot()!; const owner = epoch; const switched = imported;
		if (install && !(await dialog.confirm({ title: 'Save & install hooks?', body: switched ? 'Git will use these blocks for commits and pushes. The manager’s original files stay in place.' : 'Git will run these commands for terminal commits and pushes too. Format blocks may change staged files.', confirmLabel: 'Save & install' }))) return false;
		if (owner !== epoch) return false; saving = true;
		try {
			await api.save(definition, install, switched); if (owner !== epoch) return false;
			saved = JSON.stringify(definition); errors.save = ''; imported = false;
			const result = await api.load(); if (owner === epoch) data = result;
			return true;
		} catch (cause) { if (owner === epoch) errors.save = String(cause); return false; }
		finally { if (owner === epoch) saving = false; }
	},
	async importManager() {
		const owner = epoch;
		if (!(await dialog.confirm({ title: 'Import the current hooks?', body: 'Read their commands into blocks. The original files stay in place; installing is a separate action.', confirmLabel: 'Import' }))) return;
		if (owner !== epoch) return;
		try { const result = await api.importManager(); if (owner !== epoch) return; draft = result; imported = true; selected = draft.blocks[0]?.id ?? null; errors.save = ''; void this.checkMatches(); }
		catch (cause) { if (owner === epoch) errors.save = String(cause); }
	},
	async dryRun(stage: Stage, onstart?: () => void) {
		if (!draft || running || this.readonly) return;
		const owner = epoch; const definition = snapshot()!;
		if (!(await dialog.confirm({ title: 'Dry run on staged files?', body: 'Run these commands without committing. Format blocks can change files and restage them when enabled.', confirmLabel: 'Run hooks' }))) return;
		if (owner !== epoch) return; running = true; errors.run = ''; current = null; selectedStep = null; log = ''; onstart?.();
		try { const result = await api.dryRun(definition, stage); if (owner === epoch) { tried = result; await this.refreshRuns(true); } }
		catch (cause) { if (owner === epoch) errors.run = String(cause); }
		finally { if (owner === epoch) running = false; }
	},
	async tryMessage(message: string) {
		if (!draft || running || this.readonly) return;
		const owner = epoch; running = true;
		try { const result = await api.tryMessage(snapshot()!, message); if (owner === epoch) { tried = result; errors.run = ''; } }
		catch (cause) { if (owner === epoch) errors.run = String(cause); }
		finally { if (owner === epoch) running = false; }
	},
	async refreshRuns(newest = false) {
		const owner = epoch; const request = ++reading; if (!path) return;
		try {
			const result = await api.runs(); if (owner !== epoch || request !== reading) return;
			if (data) data.runs = result;
			errors.records = '';
			const id = newest || !current ? result[0]?.id : current.id;
			if (id) await this.openRun(id, true);
		} catch (cause) { if (owner === epoch && request === reading) errors.records = String(cause); }
	},
	async openRun(id: string, keepStep = false) {
		const owner = epoch; const request = ++picking;
		try {
			const result = await api.run(id); if (owner !== epoch || request !== picking) return;
			current = result;
			const rows = steps(result);
			if (!keepStep || !rows.some((row) => row.id === selectedStep)) selectedStep = rows.find((row) => row.state === 'running')?.id ?? rows[0]?.id ?? null;
			errors.records = ''; await this.openStep(selectedStep);
		} catch (cause) { if (owner === epoch && request === picking) errors.records = String(cause); }
	},
	async openStep(id: string | null) {
		selectedStep = id; const run = current; const owner = epoch;
		if (!run || !id) { log = ''; return; }
		try { const text = await api.log(run.id, id); if (owner === epoch && current?.id === run.id && selectedStep === id) { log = text; errors.log = ''; } }
		catch (cause) { if (owner === epoch && current?.id === run.id && selectedStep === id) errors.log = String(cause); }
	},
	async stop() {
		const run = current; const owner = epoch; if (!run || run.exit !== null) return;
		try { await api.stop(run.id); } catch (cause) { if (owner === epoch) errors.run = String(cause); }
	},
	async undoRestage() {
		const run = current; const owner = epoch; if (!run || run.exit === null) return;
		if (!(await dialog.confirm({ title: 'Undo the formatter’s restage?', body: 'Restore the saved index entries. Working files stay as they are; later staging decisions are refused.', confirmLabel: 'Undo restage' }))) return;
		if (owner !== epoch) return;
		try { await api.undoRestage(run.id); if (owner === epoch) await this.refreshRuns(); }
		catch (cause) { if (owner === epoch) errors.run = String(cause); }
	}
};
