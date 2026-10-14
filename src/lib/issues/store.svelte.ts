// SPDX-License-Identifier: GPL-3.0-or-later

/** Cache first, then revalidate. Reads and writes have independent errors;
 * every reply belongs to the repository, remote, filter and selection asked. */
import { dialog } from '$lib/ui/dialog.svelte';
import { describe } from '$lib/ui/notice.svelte';
import { watcher } from '$lib/notifications/watcher.svelte';
import * as api from './api';
import type { Hosted, Issue, Metadata, Thread, Write } from './types';

let hosts = $state<Hosted[]>([]), selected = $state(''), query = $state('is:open');
let rows = $state<Issue[]>([]), current = $state<Thread | null>(null), meta = $state<Metadata | null>(null);
let loading = $state(false), reading = $state(false), busy = $state(false), more = $state(false), provisional = $state(false);
let canWrite = $state<boolean | null>(null);
let errors = $state({ list: '', thread: '', write: '', metadata: '', cache: '', branch: '' });
let owner: string | null = null, epoch = 0, selection = 0, page = 1, commentsPage = 1, eventsPage = 1;
let unsubscribe: (() => void) | null = null;
const unique = <T extends { id: string }>(values: T[]) => [...new Map(values.map((value) => [value.id, value])).values()];

export const issues = {
	get hosts() { return hosts; }, get remote() { return selected; }, get host() { return hosts.find((host) => host.remote === selected) ?? null; },
	get query() { return query; }, get rows() { return rows; }, get current() { return current; }, get metadata() { return meta; },
	get loading() { return loading; }, get reading() { return reading; }, get busy() { return busy; }, get more() { return more; }, get provisional() { return provisional; },
	get canWrite() { return canWrite !== false && this.host?.user != null && this.host.repo.kind !== 'bitbucket'; }, get errors() { return errors; },
	async load(path: string | null) {
		this.clear(); owner = path; if (!path) return;
		const ticket = epoch; loading = true;
		try {
			const result = await api.remotes(); if (ticket !== epoch) return; hosts = result;
			selected = result.find((host) => host.remote === 'origin')?.remote ?? result[0]?.remote ?? '';
			unsubscribe = watcher.subscribe(() => this.refresh());
			if (selected) await this.read(true);
		} catch (cause) { if (ticket === epoch) errors.list = describe(cause); }
		finally { if (ticket === epoch) loading = false; }
	},
	detach() { unsubscribe?.(); unsubscribe = null; },
	clear() { this.detach(); epoch++; selection++; owner = null; hosts = []; selected = ''; rows = []; current = null; meta = null; page = 1; commentsPage = 1; eventsPage = 1; canWrite = null; query = 'is:open'; loading = false; reading = false; busy = false; more = false; provisional = false; errors = { list: '', thread: '', write: '', metadata: '', cache: '', branch: '' }; },
	async choose(remote: string) { if (remote === selected) return; epoch++; selection++; selected = remote; rows = []; current = null; meta = null; canWrite = null; reading = false; busy = false; errors = { list: '', thread: '', write: '', metadata: '', cache: '', branch: '' }; await this.read(true); },
	async filter(text: string) { if (query === text) return; epoch++; selection++; query = text; rows = []; current = null; reading = false; busy = false; await this.read(true); },
	async read(reset = false) {
		if (!owner || !selected || reading || this.host?.repo.kind === 'bitbucket') return;
		const ticket = epoch, remote = selected, text = query; reading = true;
		if (reset) { page = 1; void this.readMetadata(); }
		const number = reset ? 1 : page + 1;
		try {
			const cached = await api.cached(remote, text, number); if (ticket !== epoch) return;
			if (cached) { rows = unique(reset ? cached.issues : [...rows, ...cached.issues]); more = cached.more; provisional = true; canWrite = cached.canWrite; }
			const answer = await api.page(remote, text, number); if (ticket !== epoch) return;
			rows = unique(reset ? answer.data.issues : [...rows.filter((row) => !cached?.issues.some((old) => old.id === row.id)), ...answer.data.issues]);
			page = number; more = answer.data.more; canWrite = answer.data.canWrite; provisional = false; errors.list = ''; errors.cache = answer.cacheWarning ?? '';
		} catch (cause) { if (ticket === epoch) errors.list = describe(cause); }
		finally { if (ticket === epoch) reading = false; }
	},
	async readMetadata(refresh = false) {
		const ticket = epoch, remote = selected; if (!remote || this.host?.repo.kind === 'bitbucket') return;
		try { const answer = await api.metadata(remote, refresh); if (ticket !== epoch) return; meta = answer.data; errors.metadata = ''; if (answer.cacheWarning) errors.cache = answer.cacheWarning; }
		catch (cause) { if (ticket === epoch) errors.metadata = describe(cause); }
	},
	async refresh() { if (reading || !selected) return; await this.read(true); if (current) await this.open(current.issue.number, true); },
	async open(number: number, keep = false) {
		const ticket = epoch, picked = ++selection, remote = selected;
		if (!keep) { current = null; commentsPage = 1; eventsPage = 1; }
		try { const answer = await api.thread(remote, number); if (ticket !== epoch || picked !== selection) return; current = answer; commentsPage = 1; eventsPage = 1; errors.thread = ''; }
		catch (cause) { if (ticket === epoch && picked === selection) errors.thread = describe(cause); }
	},
	async moreThread(which: 'comments' | 'events') {
		const old = current; if (!old) return; const ticket = epoch, picked = selection;
		const nextComments = which === 'comments' ? commentsPage + 1 : commentsPage;
		const nextEvents = which === 'events' ? eventsPage + 1 : eventsPage;
		try { const answer = await api.thread(selected, old.issue.number, nextComments, nextEvents); if (ticket !== epoch || picked !== selection) return; current = { ...answer, comments: unique([...old.comments, ...answer.comments]), events: unique([...old.events, ...answer.events]) }; commentsPage = nextComments; eventsPage = nextEvents; errors.thread = ''; }
		catch (cause) { if (ticket === epoch && picked === selection) errors.thread = describe(cause); }
	},
	async send(action: Write): Promise<boolean> {
		if (!this.canWrite || busy) return false;
		const ticket = epoch, remote = selected;
		const title = action.kind === 'create' ? 'Create this issue?' : action.kind === 'comment' ? `Post a comment on #${action.number}?` : `Close issue #${action.number}?`;
		const content = action.kind === 'create' ? `${action.title}\n\n${action.body}` : action.kind === 'comment' ? action.body : 'The issue will be closed on its host.';
		const body = `${this.host?.repo.host}/${this.host?.repo.owner}/${this.host?.repo.name}\n\n${content}`;
		if (!(await dialog.confirm({ title, body, confirmLabel: action.kind === 'create' ? 'Create issue' : action.kind === 'comment' ? 'Post comment' : 'Close issue' }))) return false;
		if (ticket !== epoch) return false; busy = true;
		try { await api.write(remote, action); if (ticket !== epoch) return false; errors.write = ''; await this.refresh(); void this.readMetadata(true); return true; }
		catch (cause) { if (ticket === epoch) { errors.write = describe(cause); if (/refused the token|not authorized|insufficient scope|permission denied/i.test(errors.write)) canWrite = false; } return false; }
		finally { if (ticket === epoch) busy = false; }
	},
	async closeWithComment(body: string) { const issue = current?.issue; if (!issue) return false; if (!await this.send({ kind: 'comment', number: issue.number, body })) return false; return this.send({ kind: 'close', number: issue.number }); },
	async branch(name: string) {
		const ticket = epoch, remote = selected;
		if (!meta?.defaultBranch || busy || !name.trim()) return false;
		if (!await dialog.confirm({ title: `Create ${name}?`, body: `Start from ${meta.defaultBranch} and check out the new branch. Git refuses to overwrite local changes.`, confirmLabel: 'Create branch' })) return false;
		if (ticket !== epoch) return false; busy = true;
		try { await api.branch(remote, name); if (ticket !== epoch) return false; errors.branch = ''; return true; }
		catch (cause) { if (ticket === epoch) errors.branch = describe(cause); return false; }
		finally { if (ticket === epoch) busy = false; }
	}
};
