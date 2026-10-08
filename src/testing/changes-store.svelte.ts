// SPDX-License-Identifier: GPL-3.0-or-later

/**
 * A reactive stand-in for the working-copy store, for component tests.
 *
 * Reactive for the same reason as the other stubs: the components re-render
 * through Svelte's reactivity, and one built from plain properties would render
 * the initial value and then never change.
 *
 * Lives outside `src/lib` so it is not counted as first-party code by the
 * coverage floor.
 */

import type { FileDiff, WorkingCopy, Signing } from '$lib/types';
import type { Selection } from '$lib/changes/store.svelte';

const EMPTY: WorkingCopy = { staged: [], unstaged: [], conflicted: [] };

let work = $state<WorkingCopy>(EMPTY);
let loaded = $state(false);
let error = $state<string | null>(null);
let loading = $state(false);
let selection = $state<Selection | null>(null);
let file = $state<FileDiff | null>(null);
let fileError = $state<string | null>(null);
let fileLoading = $state(false);
let subject = $state('');
let body = $state('');
let amend = $state(false);
/** FEAT-019: what the message box says about signing, or null to say nothing. */
let signing = $state<Signing | null>(null);
let busy = $state(false);
let writeError = $state<string | null>(null);

/** Calls the components made, for assertions. */
export const calls = {
	staged: [] as string[][],
	unstaged: [] as string[][],
	hunks: [] as Array<{ index: number; header: string }>,
	discarded: [] as string[][],
	discardedHunks: [] as Array<{ index: number; header: string }>,
	opened: [] as Selection[],
	commits: 0,
	loads: 0,
	amends: [] as boolean[]
};

export const control = {
	setWork(next: WorkingCopy) {
		work = next;
		loaded = true;
	},
	setSelection(next: Selection | null) {
		selection = next;
	},
	setFile(next: FileDiff | null) {
		file = next;
	},
	setFileError(next: string | null) {
		fileError = next;
	},
	setFileLoading(next: boolean) {
		fileLoading = next;
	},
	setError(next: string | null) {
		error = next;
	},
	setLoaded(next: boolean) {
		loaded = next;
	},
	setLoading(next: boolean) {
		loading = next;
	},
	setBusy(next: boolean) {
		busy = next;
	},
	setWriteError(next: string | null) {
		writeError = next;
	},
	setSubject(next: string) {
		subject = next;
	},
	setAmend(next: boolean) {
		amend = next;
	},
	setSigning(next: Signing | null) {
		signing = next;
	},
	reset() {
		work = EMPTY;
		loaded = false;
		error = null;
		loading = false;
		selection = null;
		file = null;
		fileError = null;
		fileLoading = false;
		subject = '';
		body = '';
		amend = false;
		signing = null;
		busy = false;
		writeError = null;
		calls.staged = [];
		calls.unstaged = [];
		calls.hunks = [];
		calls.discarded = [];
		calls.discardedHunks = [];
		calls.opened = [];
		calls.commits = 0;
		calls.loads = 0;
		calls.amends = [];
	}
};

export const changes = {
	get work() {
		return work;
	},
	get loaded() {
		return loaded;
	},
	get error() {
		return error;
	},
	get loading() {
		return loading;
	},
	get selection() {
		return selection;
	},
	get file() {
		return file;
	},
	get fileError() {
		return fileError;
	},
	get fileLoading() {
		return fileLoading;
	},
	get subject() {
		return subject;
	},
	get body() {
		return body;
	},
	get amend() {
		return amend;
	},
	get signing(): Signing | null {
		return signing;
	},
	get busy() {
		return busy;
	},
	get writeError() {
		return writeError;
	},
	get canCommit() {
		return !busy && subject.trim() !== '' && work.conflicted.length === 0 &&
			(work.staged.length > 0 || amend);
	},
	setSubject(next: string) {
		subject = next;
	},
	setBody(next: string) {
		body = next;
	},
	async setAmend(next: boolean) {
		amend = next;
		calls.amends.push(next);
	},
	async load() {
		calls.loads += 1;
	},
	openFirst() {},
	open(next: Selection) {
		calls.opened.push(next);
		selection = next;
	},
	async run(operation: () => Promise<void>) {
		await operation();
		return true;
	},
	async stage(paths: string[]) {
		calls.staged.push(paths);
		return true;
	},
	async unstage(paths: string[]) {
		calls.unstaged.push(paths);
		return true;
	},
	async hunk(index: number, header: string) {
		calls.hunks.push({ index, header });
		return true;
	},
	async discard(paths: string[]) {
		calls.discarded.push(paths);
		return true;
	},
	async discardHunk(index: number, header: string) {
		if (selection?.side !== 'unstaged') return false;
		calls.discardedHunks.push({ index, header });
		return true;
	},
	async commit() {
		calls.commits += 1;
		return true;
	},
	clear() {
		control.reset();
	}
};
