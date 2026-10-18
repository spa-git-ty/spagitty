// SPDX-License-Identifier: GPL-3.0-or-later

import * as api from './api';
import { watcher } from '$lib/notifications/watcher.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import { describe } from '$lib/ui/notice.svelte';
import type { Hosted, LiveJob, Page, Preview, Simulation, Watches, Workflow, Write } from './types';

let owner: string | null = null, epoch = 0, previewTicket = 0, jobTicket = 0, logTicket = 0;
let unsubscribe: (() => void) | null = null, period = 0;
let attached = false, sourceTicket = 0;
let nextWatches = 0, graphReading = false;
let checked: Record<string, number> = {};
let hosts = $state<Hosted[]>([]), remote = $state(''), view = $state<'preview' | 'live'>('preview');
let local = $state<Preview | null>(null), file = $state(''), picked = $state(''), loading = $state(false), previewing = $state(false);
let simulation = $state<Simulation>({ event: 'push', reference: '', base: '', defaultBranch: '', changed: [], changesKnown: false });
let mode = $state('working'), from = $state('HEAD~1'), to = $state('HEAD');
let watches = $state<Watches | null>(null), personal = $state<string[]>([]), branch = $state(''), lists = $state<Record<string, Page>>({});
let runId = $state(0), jobs = $state<LiveJob[]>([]), selectedJob = $state(0), log = $state(''), moreJobs = $state(false), page = 1;
let reading = $state(false), busy = $state(false), canWrite = $state(true), updated = $state(0), clock = $state(Date.now());
let errors = $state({ preview: '', hosts: '', runs: '', jobs: '', log: '', write: '', watches: '', cache: '' });
let lint = $state(''), source = $state<{ file: string; line: number; text: string } | null>(null), graph = $state<Workflow | null>(null), graphWarning = $state('');
const unique = (values: string[]) => [...new Set(values.filter(Boolean))];
function schedule() {
	const active = Object.values(lists).some((p) => p.runs.some((r) => r.state !== 'completed'));
	const next = attached && view === 'live' ? active ? 5000 : 60_000 : 0;
	if (next === period) return; unsubscribe?.(); unsubscribe = null; period = next;
	if (next) unsubscribe = watcher.subscribe(() => pipelines.refresh(false), next);
}
const currentRun = () => lists[branch]?.runs.find((r) => r.id === runId) ?? null;
export const pipelines = {
	get repo() { return owner; }, get hosts() { return hosts; }, get remote() { return remote; }, get host() { return hosts.find((h) => h.remote === remote); },
	get view() { return view; }, get local() { return local; }, get workflow() { return local?.workflows.find((w) => w.file === file) ?? null; }, get file() { return file; },
	get job() { return this.workflow?.jobs.find((j) => j.id === picked) ?? null; }, get loading() { return loading; }, get previewing() { return previewing; },
	get simulation() { return simulation; }, get mode() { return mode; }, get from() { return from; }, get to() { return to; }, get errors() { return errors; },
	get branches() { return unique([watches?.defaultBranch ?? '', ...personal]); }, get requests() { return unique(watches?.requests ?? []); }, get branch() { return branch; },
	get lists() { return lists; }, get runs() { return lists[branch]?.runs ?? []; }, get run() { return currentRun(); }, get jobs() { return jobs; },
	get liveJob() { return jobs.find((j) => j.id === selectedJob) ?? null; }, get log() { return log; }, get reading() { return reading; }, get busy() { return busy; },
	get canWrite() { return canWrite; }, get moreJobs() { return moreJobs; }, get lint() { return lint; }, get source() { return source; },
	get updated() { return updated; }, get age() { return updated ? Math.max(0, Math.floor((clock - updated) / 1000)) : null; },
	get graph() { return graph; }, get graphWarning() { return graphWarning; },
	clear() { this.detach(); epoch++; previewTicket++; jobTicket++; logTicket++; sourceTicket++; nextWatches = 0; graphReading = false; owner = null; hosts = []; remote = ''; local = null; file = ''; picked = ''; watches = null; personal = []; branch = ''; lists = {}; checked = {}; runId = 0; jobs = []; selectedJob = 0; log = ''; graph = null; graphWarning = ''; loading = false; previewing = false; reading = false; busy = false; canWrite = true; moreJobs = false; page = 1; updated = 0; lint = ''; source = null; mode = 'working'; from = 'HEAD~1'; to = 'HEAD'; simulation = { event: 'push', reference: '', base: '', defaultBranch: '', changed: [], changesKnown: false }; errors = { preview: '', hosts: '', runs: '', jobs: '', log: '', write: '', watches: '', cache: '' }; },
	detach() { attached = false; unsubscribe?.(); unsubscribe = null; period = 0; },
	async load(path: string | null) {
		if (owner === path) { attached = !!path; if (path) await this.preview(); schedule(); return; }
		this.clear(); owner = path; attached = !!path; if (!path) return; const ticket = epoch; loading = true;
		// The host list is not waited behind the workflow files: Live and the host
		// switch need it, and a large preview should not hold them back.
		const previewed = this.preview();
		try { const answer = await api.remotes(); if (ticket !== epoch) return; hosts = answer; remote = hosts.find((h) => h.repo.kind !== 'bitbucket')?.remote ?? hosts[0]?.remote ?? ''; if (view === 'live') await this.readWatches(); }
		catch (cause) { if (ticket === epoch) errors.hosts = describe(cause); }
		finally { await previewed; if (ticket === epoch) { loading = false; schedule(); } }
	},
	async setView(next: 'preview' | 'live') { view = next; schedule(); if (next === 'live' && owner && !watches) await this.readWatches(); },
	async choose(remoteName: string) { if (remote === remoteName) return; epoch++; jobTicket++; logTicket++; remote = remoteName; watches = null; lists = {}; branch = ''; jobs = []; runId = 0; selectedJob = 0; log = ''; reading = false; busy = false; updated = 0; canWrite = true; errors.runs = ''; errors.jobs = ''; errors.log = ''; errors.write = ''; checked = {}; graph = null; graphWarning = ''; if (view === 'live') await this.readWatches(); else await this.preview(); },
	async preview(patch: Partial<Simulation> = {}, changedMode = mode, start = from, end = to) {
		simulation = { ...simulation, ...patch }; mode = changedMode; from = start; to = end;
		if (!owner) return; const ticket = ++previewTicket, repository = epoch; previewing = true;
		try { const answer = await api.preview($state.snapshot(simulation), mode, from, to); if (ticket !== previewTicket || repository !== epoch) return; local = answer; simulation = answer.simulation; if (!answer.workflows.some((w) => w.file === file)) file = answer.workflows[0]?.file ?? ''; if (!this.workflow?.jobs.some((j) => j.id === picked)) picked = this.workflow?.jobs[0]?.id ?? ''; errors.preview = ''; }
		catch (cause) { if (ticket === previewTicket && repository === epoch) errors.preview = describe(cause); }
		finally { if (ticket === previewTicket && repository === epoch) previewing = false; }
	},
	selectFile(name: string) { file = name; picked = this.workflow?.jobs[0]?.id ?? ''; }, selectJob(id: string) { picked = id; },
	async openSource(file: string, line = 1) { const ticket = epoch, asked = ++sourceTicket; try { const text = await api.source(file); if (ticket === epoch && asked === sourceTicket) source = { file, line, text }; } catch (cause) { if (ticket === epoch && asked === sourceTicket) errors.preview = describe(cause); } },
	closeSource() { sourceTicket++; source = null; },
	async readWatches() {
		if (!owner || !remote || this.host?.repo.kind === 'bitbucket') return; const ticket = epoch, selected = remote;
		try { const prefs = await api.preferences(selected); if (ticket !== epoch) return; personal = prefs.branches; const answer = await api.watches(selected); if (ticket !== epoch) return; watches = answer.data; nextWatches = Date.now() + 60_000; errors.cache = answer.cacheWarning ?? ''; errors.watches = ''; if (!branch) branch = watches.defaultBranch || personal[0] || watches.requests[0] || ''; await this.refresh(); }
		catch (cause) { if (ticket === epoch) { errors.watches = describe(cause); if (!branch && personal.length) { branch = personal[0]; await this.refresh(); } } }
	},
	async watch(name: string, remove = false) {
		name = name.trim(); if (!name || busy || !remote) return; const ticket = epoch;
		try { const answer = await api.preferences(remote, remove ? personal.filter((b) => b !== name) : unique([...personal, name])); if (ticket !== epoch) return; personal = answer.branches; if (!remove) branch = name; errors.watches = ''; await this.refresh(); }
		catch (cause) { if (ticket === epoch) errors.watches = describe(cause); }
	},
	async chooseBranch(name: string) { if (branch === name) return; branch = name; jobTicket++; logTicket++; jobs = []; runId = 0; selectedJob = 0; log = ''; await this.refresh(); },
	async refresh(force = true) {
		clock = Date.now(); if (view !== 'live' || reading || !owner || !remote || this.host?.repo.kind === 'bitbucket') return;
		const ticket = epoch, selected = remote; reading = true; let failed = false, refreshed = false;
		try { if (clock >= nextWatches) { nextWatches = clock + 60_000; try { const answer = await api.watches(selected); if (ticket !== epoch) return; watches = answer.data; errors.watches = ''; } catch (cause) { if (ticket !== epoch) return; errors.watches = describe(cause); } }
		for (const name of unique([branch, ...this.branches, ...this.requests])) {
			const every = lists[name]?.runs.some((r) => r.state !== 'completed') ? 5000 : 60_000;
			if (!force && clock - (checked[name] ?? 0) < every) continue;
			const cached = await api.cached(selected, name); if (ticket !== epoch) return; if (cached && !lists[name]) lists = { ...lists, [name]: cached };
			try { const answer = await api.runs(selected, name); if (ticket !== epoch) return; lists = { ...lists, [name]: answer.data }; checked[name] = Date.now(); if (name === branch) refreshed = true; errors.cache = answer.cacheWarning ?? ''; }
			catch (cause) { if (ticket !== epoch) return; failed = true; errors.runs = describe(cause); continue; }
		}
		if (ticket !== epoch) return; if (refreshed) updated = Date.now(); if (!failed) errors.runs = '';
		if (!this.runs.some((r) => r.id === runId)) await this.openRun(this.runs[0]?.id ?? 0); else if (runId) { if (!graph) void this.readGraph(); await this.readJobs(); }
		} catch (cause) { if (ticket === epoch) errors.runs = describe(cause); }
		finally { if (ticket === epoch) { reading = false; schedule(); } }
	},
	async openRun(id: number) { runId = id; jobTicket++; logTicket++; page = 1; jobs = []; selectedJob = 0; log = ''; graph = null; graphWarning = ''; graphReading = false; errors.log = ''; if (id) { void this.readGraph(); await this.readJobs(); } },
	async readGraph() { if (graphReading || !runId) return; const ticket = epoch, id = runId; graphReading = true; try { const answer = await api.graph(remote, id); if (ticket !== epoch || id !== runId) return; graph = answer.data; graphWarning = ''; } catch (cause) { if (ticket === epoch && id === runId) graphWarning = `Dependencies unavailable: ${describe(cause)}`; } finally { if (ticket === epoch && id === runId) graphReading = false; } },
	async readJobs(more = false) {
		const ticket = epoch, selected = ++jobTicket, id = runId, remoteName = remote; if (!id) return; const end = more ? page + 1 : page;
		try { let incoming = more ? [...jobs] : [], hasMore = false, readPage = 1;
			for (let number = more ? end : 1; number <= end; number++) { const answer = await api.jobs(remoteName, id, number); if (ticket !== epoch || selected !== jobTicket || id !== runId) return; incoming = [...incoming, ...answer.data.jobs]; readPage = number; hasMore = answer.data.more; if (!hasMore) break; }
			jobs = [...new Map(incoming.map((j) => [j.id, j])).values()]; page = readPage; moreJobs = hasMore; errors.jobs = ''; if (!jobs.some((j) => j.id === selectedJob)) selectedJob = jobs.find((j) => j.state === 'running')?.id ?? jobs[0]?.id ?? 0; await this.readLog(); }
		catch (cause) { if (ticket === epoch && selected === jobTicket) errors.jobs = describe(cause); }
	},
	async selectLiveJob(id: number) { selectedJob = id; log = ''; errors.log = ''; await this.readLog(); },
	async readLog() {
		const job = this.liveJob, ticket = epoch, selected = ++logTicket; if (!job) return;
		if (this.host?.repo.kind === 'gitHub' && job.state !== 'completed') { log = ''; errors.log = ''; return; }
		try { const answer = await api.log(remote, job.id); if (ticket !== epoch || selected !== logTicket || job.id !== selectedJob) return; log = answer; errors.log = ''; }
		catch (cause) { if (ticket === epoch && selected === logTicket) errors.log = describe(cause); }
	},
	async send(kind: 'cancel' | 'rerunFailed' | 'lint') {
		const run = currentRun(), ticket = epoch, selected = remote; if (busy || !this.host || (kind !== 'lint' && (!run || !canWrite))) return false;
		const title = kind === 'lint' ? 'Send CI configuration to GitLab lint?' : kind === 'cancel' ? `Cancel pipeline #${run!.id}?` : `Re-run failed jobs in #${run!.id}?`;
		const host = this.host.repo;
		if (!await dialog.confirm({ title, body: `${host.host}/${host.owner}/${host.name}\n\n${kind === 'lint' ? 'Send the current .gitlab-ci.yml to this host. Its includes may be resolved by GitLab.' : 'This changes the run on its host.'}`, confirmLabel: kind === 'lint' ? 'Send to lint' : kind === 'cancel' ? 'Cancel pipeline' : 'Re-run failed', danger: kind === 'cancel' })) return false;
		if (ticket !== epoch || (kind !== 'lint' && run?.id !== runId)) return false; busy = true;
		const action: Write = kind === 'lint' ? { kind, content: '' } : { kind, id: run!.id };
		try { const answer = await api.write(selected, action); if (ticket !== epoch) return false; errors.write = ''; if (kind === 'lint') lint = JSON.stringify(answer, null, 2); else await this.refresh(); return true; }
		catch (cause) { if (ticket === epoch) { errors.write = describe(cause); if (/Grant Actions: write|Grant the api scope/.test(errors.write)) canWrite = false; } return false; }
		finally { if (ticket === epoch) busy = false; }
	}
};
