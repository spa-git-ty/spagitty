// SPDX-License-Identifier: GPL-3.0-or-later
import { dialog } from '$lib/ui/dialog.svelte';
import { describe } from '$lib/ui/notice.svelte';
import { watcher } from '$lib/notifications/watcher.svelte';
import * as api from './api';
import { empty, type Artifact, type Asset, type Download, type FileTransfer, type Hosted, type Preferences, type Release, type Write } from './types';

let hosts = $state<Hosted[]>([]), remote = $state('');
let rows = $state<Release[]>([]), preferences = $state<Preferences>({ hidden: [], drafts: [] });
let draft = $state<Release | null>(null), selected = $state('');
let loading = $state(false), reading = $state(false), busy = $state(false), more = $state(false), provisional = $state(false), readonly = $state(false);
let errors = $state({ read: '', write: '', cache: '', notes: '', files: '', changelog: '' });
let files = $state<FileTransfer[]>([]), artifacts = $state<Artifact[]>([]);
let downloads = $state<Download[]>([]);
let pending = $state<{ branch: string; expected: string; commit: string } | null>(null);
let filter = $state<'all' | 'drafts' | 'prereleases'>('all'), showHidden = $state(false);
let epoch = 0, noteRequest = 0, path: string | null = null, page = 1, fileKey = 0, stop: (() => void) | null = null;
const clone = (release: Release): Release => structuredClone($state.snapshot(release));
const unique = (values: Release[]) => [...new Map(values.map((value) => [value.id, value])).values()];

export const releases = {
	get hosts() { return hosts; }, get remote() { return remote; }, get host() { return hosts.find((h) => h.remote === remote) ?? null; },
	get rows() { return unique([...preferences.drafts, ...rows]); },
	get visible() { return this.rows.filter((release) => (showHidden || !preferences.hidden.includes(release.id)) && (filter === 'all' || (filter === 'drafts' ? release.draft : release.prerelease))); },
	get draft() { return draft; }, get selected() { return selected; }, get hidden() { return preferences.hidden; },
	get filter() { return filter; }, set filter(value: typeof filter) { filter = value; },
	get showHidden() { return showHidden; }, set showHidden(value: boolean) { showHidden = value; },
	get loading() { return loading; }, get reading() { return reading; }, get busy() { return busy; }, get more() { return more; }, get provisional() { return provisional; },
	get canWrite() { return !readonly && this.host?.user != null && this.host.repo.kind !== 'bitbucket'; },
	get errors() { return errors; }, get files() { return files; }, get artifacts() { return artifacts; }, get downloads() { return downloads; }, get pendingChangelog() { return pending; },
	detach() { stop?.(); stop = null; },
	clear() {
		this.detach(); epoch++; path = null; hosts = []; remote = ''; rows = []; preferences = { hidden: [], drafts: [] }; draft = null; selected = '';
		loading = false; reading = false; busy = false; more = false; provisional = false; readonly = false; page = 1; files = []; artifacts = []; downloads = []; pending = null;
		errors = { read: '', write: '', cache: '', notes: '', files: '', changelog: '' };
	},
	async load(repo: string | null) {
		this.clear(); path = repo; if (!repo) return; const ticket = epoch; loading = true;
		try {
			const answer = await api.remotes(); if (ticket !== epoch) return; hosts = answer;
			remote = answer.find((h) => h.remote === 'origin')?.remote ?? answer[0]?.remote ?? '';
			stop = watcher.subscribe(() => this.read(true)); if (remote) await this.read(true);
		} catch (cause) { if (ticket === epoch) errors.read = describe(cause); }
		finally { if (ticket === epoch) loading = false; }
	},
	async choose(value: string) {
		if (value === remote || !(await this.canLeave())) return;
		epoch++; remote = value; rows = []; draft = null; selected = ''; files = []; artifacts = []; downloads = []; pending = null; readonly = false; reading = false; busy = false; preferences = { hidden: [], drafts: [] };
		await this.read(true);
	},
	async read(reset = false) {
		if (!path || !remote || reading || this.host?.repo.kind === 'bitbucket') return;
		const ticket = epoch, number = reset ? 1 : page + 1; reading = true;
		try {
			if (reset) { const local = await api.preferences(remote); if (ticket !== epoch) return; preferences = local; }
			const saved = await api.cached(remote, number); if (ticket !== epoch) return;
			if (saved) { rows = unique(reset ? saved.releases : [...rows, ...saved.releases]); more = saved.more; provisional = true; }
			const answer = await api.page(remote, number); if (ticket !== epoch) return;
			rows = unique(reset ? answer.data.releases : [...rows, ...answer.data.releases]); page = number; more = answer.data.more; provisional = false;
			errors.read = ''; errors.cache = answer.cacheWarning ?? '';
		} catch (cause) { if (ticket === epoch) errors.read = describe(cause); }
		finally { if (ticket === epoch) reading = false; }
	},
	get dirty() {
		if (!draft) return false;
		const original = this.rows.find((release) => release.id === selected);
		return !original ? Boolean(draft.tag || draft.title || draft.body || files.length) : JSON.stringify($state.snapshot(draft)) !== JSON.stringify($state.snapshot(original));
	},
	async canLeave() {
		if (busy || files.some((file) => file.state === 'uploading') || downloads.some((file) => file.busy)) return false;
		return !this.dirty || await dialog.confirm({ title: 'Discard release edits?', body: 'The unsaved release text and queued files will be discarded.', confirmLabel: 'Discard', danger: true });
	},
	async select(release: Release) { if (!(await this.canLeave())) return; selected = release.id; draft = clone(release); files = []; artifacts = []; downloads = []; pending = null; },
	async create() { if (!(await this.canLeave())) return; selected = ''; draft = empty(); files = []; artifacts = []; downloads = []; pending = null; },
	async hide(release: Release) {
		const ticket = epoch;
		try { const answer = await api.hide(remote, release.id, !preferences.hidden.includes(release.id)); if (ticket === epoch) preferences = answer; }
		catch (cause) { if (ticket === epoch) errors.cache = describe(cause); }
	},
	async mutation(action: Write, title: string, body: string, danger = false): Promise<unknown | false> {
		if (!this.canWrite || busy) return false;
		const ticket = epoch, chosen = remote;
		if (!(await dialog.confirm({ title, body: `${this.host?.repo.host}/${this.host?.repo.owner}/${this.host?.repo.name}\n\n${body}`, confirmLabel: title, danger })) || ticket !== epoch) return false;
		busy = true;
		try { const answer = await api.write(chosen, action); if (ticket !== epoch) return false; errors.write = ''; return answer ?? null; }
		catch (cause) { if (ticket === epoch) { errors.write = describe(cause); if (/forbidden|403|permission|read.only/i.test(errors.write)) readonly = true; } return false; }
		finally { if (ticket === epoch) busy = false; }
	},
	async save(publish: boolean, writeChangelog = false, branch = '') {
		if (!draft || !draft.tag.trim() || !draft.target.trim() || !this.canWrite) return false;
		const ticket = epoch;
		if (publish && writeChangelog) {
			if (!pending) {
				if (!(await dialog.confirm({ title: 'Commit release changelog?', body: `Write ${draft.tag} to CHANGELOG.md on ${branch || draft.target}. Repository hooks run before the commit. Existing work must be clean.`, confirmLabel: 'Commit changelog' })) || ticket !== epoch) return false;
				busy = true;
				try {
					const expected = await api.target(remote, branch || draft.target); if (ticket !== epoch) return false;
					const commit = await api.changelog(remote, branch || draft.target, expected, draft.tag, draft.body); if (ticket !== epoch) return false;
					pending = { branch: branch || draft.target, expected, commit }; draft.target = commit; errors.changelog = '';
				} catch (cause) { if (ticket === epoch) errors.changelog = describe(cause); return false; }
				finally { if (ticket === epoch) busy = false; }
			}
			if (!(await dialog.confirm({ title: 'Push changelog commit?', body: `Push ${pending.commit.slice(0, 12)} to ${remote}/${pending.branch} before creating the release tag. A moved host branch stops the push.`, confirmLabel: 'Push changelog' })) || ticket !== epoch) return false;
			busy = true;
			try { await api.pushTarget(remote, pending.branch, pending.expected, pending.commit); if (ticket !== epoch) return false; pending = null; errors.changelog = ''; }
			catch (cause) { if (ticket === epoch) errors.changelog = describe(cause); return false; }
			finally { if (ticket === epoch) busy = false; }
		}
		const submitted = clone(draft), title = publish ? (draft.draft ? 'Publish release' : 'Save release') : 'Save draft';
		const answer = await this.mutation({ kind: 'save', release: submitted, publish }, title, `${submitted.tag}\nTarget: ${submitted.target}\n${submitted.title}\n\n${submitted.body}`);
		if (answer === false || ticket !== epoch) return false;
		await this.read(true); if (ticket !== epoch) return false;
		const saved = this.rows.find((release) => release.tag === submitted.tag);
		if (saved) { selected = saved.id; draft = clone(saved); }
		return true;
	},
	async remove(deleteTag = false) {
		if (!draft || !draft.id) return; const ticket = epoch, old = clone(draft);
		const answer = await this.mutation({ kind: 'delete', id: old.id, tag: old.tag }, 'Delete release', `Delete ${old.tag} and its release attachments.${this.host?.repo.kind === 'gitLab' ? ' Uploaded packages remain in the registry.' : ''} Its Git tag remains unless separately confirmed.`, true);
		if (answer === false || ticket !== epoch) return;
		rows = rows.filter((release) => release.id !== old.id); preferences.drafts = preferences.drafts.filter((release) => release.id !== old.id); draft = null; selected = '';
		if (deleteTag && !old.local) await this.mutation({ kind: 'deleteTag', tag: old.tag }, 'Delete tag', `Also delete the remote Git tag ${old.tag}. This cannot be undone here.`, true);
	},
	async removeAsset(asset: Asset) {
		if (!draft || !asset.removable) return; const ticket = epoch, id = draft.id;
		if (draft.local) { draft.assets = draft.assets.filter((a) => a.id !== asset.id); return; }
		const result = await this.mutation({ kind: 'removeAsset', releaseId: draft.id, tag: draft.tag, assetId: asset.id }, 'Remove file', `Remove ${asset.name} from ${draft.tag}.${this.host?.repo.kind === 'gitLab' ? ' This removes the release link; the uploaded package remains in the registry.' : ''}`, true);
		if (result !== false && ticket === epoch && draft?.id === id) draft.assets = draft.assets.filter((a) => a.id !== asset.id);
	},
	async generate(since: string, host = false) {
		if (!draft) return; const ticket = epoch, id = selected, request = ++noteRequest, previous = draft.body;
		try {
			const answer = host ? await this.mutation({ kind: 'generateNotes', tag: draft.tag, target: draft.target, since }, 'Generate host notes', `Ask GitHub to generate notes for ${draft.tag}.`) : await api.notes(since, draft.target);
			if (answer === false || ticket !== epoch || id !== selected || request !== noteRequest || !draft || draft.body !== previous) return;
			draft.body = host ? String((answer as { body?: string }).body ?? '') : String(answer); errors.notes = '';
		} catch (cause) { if (ticket === epoch) errors.notes = describe(cause); }
	},
	queue(paths: string[]) { for (const path of paths) if (!files.some((file) => file.path === path)) files.push({ key: ++fileKey, path, name: path.split(/[\\/]/).pop() ?? path, sent: 0, total: 0, state: 'queued', error: '', asset: null }); },
	forgetFile(key: number) { files = files.filter((file) => file.key !== key || file.state === 'uploading'); },
	async upload(key: number) {
		const file = files.find((file) => file.key === key); if (!file || !draft || file.state === 'uploading' || !this.canWrite) return;
		if (!draft.id && !draft.local) { errors.files = 'Save the draft before uploading files.'; return; }
		const ticket = epoch, id = selected, destination = clone(draft);
		if (!(await dialog.confirm({ title: 'Upload release file?', body: `Upload ${file.name} to ${this.host?.repo.host}/${this.host?.repo.owner}/${this.host?.repo.name} for ${destination.tag}.`, confirmLabel: 'Upload file' })) || ticket !== epoch) return;
		file.state = 'uploading'; file.error = ''; file.sent = 0;
		try {
			let asset = file.asset;
			if (!asset) asset = await api.upload(remote, destination, file.path, (update) => { if (ticket === epoch) { file.sent = update.sent; file.total = update.total; } });
			if (ticket !== epoch || selected !== id || !draft) return; file.asset = asset;
			if (this.host?.repo.kind === 'gitLab' && !draft.local) {
				const linked = await this.mutation({ kind: 'linkAsset', tag: draft.tag, name: asset.name, url: asset.url }, 'Link uploaded file', `The package was uploaded. Attach its link ${asset.name} to ${draft.tag}.`);
				if (linked === false) { file.state = 'error'; file.error = 'Package uploaded. Retry to link it without uploading it again.'; return; }
				const raw = linked as { id: number; direct_asset_url?: string; url?: string };
				asset = { ...asset, id: String(raw.id), url: raw.direct_asset_url ?? raw.url ?? asset.url };
			}
			draft.assets = [...draft.assets.filter((old) => old.id !== asset.id), asset]; file.state = 'uploaded'; errors.files = '';
			if (draft.local) await this.save(false);
		} catch (cause) { if (ticket === epoch) { file.state = 'error'; file.error = describe(cause); } }
	},
	async readArtifacts() {
		if (!draft) return; const ticket = epoch, id = selected, target = draft.target, chosen = remote;
		try { const commit = await api.target(chosen, target); if (ticket !== epoch || draft?.target !== target) return; const answer = await api.artifacts(chosen, commit); if (ticket === epoch && id === selected && draft?.target === target) { artifacts = answer; errors.files = ''; } }
		catch (cause) { if (ticket === epoch) errors.files = describe(cause); }
	},
	async importArtifact(artifact: Artifact) {
		const ticket = epoch, id = selected;
		let download = downloads.find((file) => file.id === artifact.id);
		if (download?.busy) return;
		if (!download) { downloads.push({ id: artifact.id, sent: 0, total: artifact.size ?? 0, busy: false, error: '', path: null }); download = downloads[downloads.length - 1]; }
		const transfer = download; transfer.busy = true; transfer.error = '';
		try {
			const downloaded = transfer.path ?? await api.download(remote, artifact, (update) => { if (ticket === epoch) { transfer.sent = update.sent; transfer.total = update.total; } });
			if (ticket !== epoch || id !== selected) return; transfer.path = downloaded; this.queue([downloaded]);
		} catch (cause) { if (ticket === epoch) transfer.error = describe(cause); }
		finally { if (ticket === epoch) transfer.busy = false; }
	}
};
