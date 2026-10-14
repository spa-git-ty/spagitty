// SPDX-License-Identifier: GPL-3.0-or-later
import { beforeEach, expect, it, vi } from 'vitest';
import { click, flushSync, render } from '../../testing/mount';
import { dialog } from '$lib/ui/dialog.svelte';
import type { Hosted, Job, LiveJob, Preview, Run, Workflow } from './types';
vi.mock('./api', () => ({ remotes: vi.fn(), preview: vi.fn(), source: vi.fn(), preferences: vi.fn(), watches: vi.fn(), cached: vi.fn(), runs: vi.fn(), jobs: vi.fn(), log: vi.fn(), write: vi.fn(), graph: vi.fn() }));
vi.mock('$lib/notifications/watcher.svelte', () => ({ watcher: { subscribe: vi.fn(() => vi.fn()) } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from './api';
import { watcher } from '$lib/notifications/watcher.svelte';
import { pipelines } from './store.svelte';
import PipelinesScreen from './PipelinesScreen.svelte';
import PipelineGraph from './PipelineGraph.svelte';
import { layout } from './layout';
import { liveGraph } from './live';
const host: Hosted = { remote: 'origin', repo: { kind: 'gitHub', host: 'github.com', owner: 'team', name: 'app' }, user: 'ada' };
const job = (id: string, patch: Partial<Job> = {}): Job => ({ id, name: id, stage: '', needs: [], depth: 0, condition: '', matrix: [], environment: '', services: [], steps: [{ name: 'Test', command: 'cargo test', condition: '' }], runsOn: ['linux'], timeout: '', secrets: [], file: '.github/workflows/ci.yml', line: 8, state: 'runs', reason: 'matches', ...patch });
const workflow: Workflow = { file: '.github/workflows/ci.yml', name: 'CI', kind: 'gitHub', triggers: ['push'], stages: [], jobs: [job('build', { matrix: [{ os: 'linux' }, { os: 'windows' }] }), job('test', { needs: ['build'], depth: 1 })], problems: [], sourceFiles: ['.github/workflows/ci.yml'] };
const preview: Preview = { workflows: [workflow], changed: ['src/main.rs'], changesWarning: null, simulation: { event: 'push', reference: 'main', base: 'main', defaultBranch: 'main', changed: ['src/main.rs'], changesKnown: true } };
const run: Run = { id: 3, name: 'CI', branch: 'main', commit: 'a'.repeat(40), event: 'push', state: 'running', result: '', started: 10, ended: 0, duration: null, url: 'https://github.com/team/app/actions/runs/3' };
const liveJob: LiveJob = { id: 5, name: 'test', stage: '', state: 'running', result: '', runner: 'linux', started: 10, ended: 0, duration: null, url: '', steps: [{ number: 1, name: 'Test', state: 'in_progress', result: '', started: 10, ended: 0 }] };
async function settle() { for (let n = 0; n < 20; n++) await Promise.resolve(); flushSync(); }
beforeEach(() => {
	pipelines.clear(); void pipelines.setView('preview'); dialog.dismiss(); vi.clearAllMocks();
	vi.mocked(api.preview).mockResolvedValue(structuredClone(preview)); vi.mocked(api.remotes).mockResolvedValue([host]);
	vi.mocked(api.preferences).mockResolvedValue({ branches: [] }); vi.mocked(api.watches).mockResolvedValue({ data: { defaultBranch: 'main', branches: [], requests: ['feature/test'] }, cacheWarning: null });
	vi.mocked(api.cached).mockResolvedValue(null); vi.mocked(api.runs).mockResolvedValue({ data: { runs: [run], etag: 'v1', more: false }, cacheWarning: null });
	vi.mocked(api.jobs).mockResolvedValue({ data: { jobs: [liveJob], page: 1, more: false, etag: null }, cacheWarning: null });
	vi.mocked(api.log).mockResolvedValue('build output'); vi.mocked(api.graph).mockResolvedValue({ data: workflow, cacheWarning: null }); vi.mocked(api.write).mockResolvedValue({ status: 'ok' });
});
it('preview reads local configuration without any host run or credential request', async () => {
	await pipelines.load('/repo'); expect(pipelines.job?.id).toBe('build'); expect(api.runs).not.toHaveBeenCalled(); expect(api.watches).not.toHaveBeenCalled();
	await pipelines.preview({ event: 'tag', reference: 'v3' }, 'range', 'HEAD~2', 'HEAD'); expect(api.preview).toHaveBeenLastCalledWith(expect.objectContaining({ event: 'tag', reference: 'v3' }), 'range', 'HEAD~2', 'HEAD');
});
it('a late simulation cannot overwrite a newer selection or repository', async () => {
	await pipelines.load('/repo'); let finish!: (answer: Preview) => void; vi.mocked(api.preview).mockReturnValueOnce(new Promise((done) => { finish = done; }));
	const old = pipelines.preview({ reference: 'old' }); await pipelines.preview({ reference: 'new' }); finish({ ...preview, changed: ['old'] }); await old; expect(pipelines.local?.changed).toEqual(['src/main.rs']);
	const pending = pipelines.preview(); pipelines.clear(); await pending; expect(pipelines.local).toBeNull();
});
it('live watches defaults and open requests and switches the shared clock to five seconds', async () => {
	await pipelines.load('/repo'); await pipelines.setView('live'); await settle(); expect(pipelines.requests).toEqual(['feature/test']); expect(api.runs).toHaveBeenCalledWith('origin', 'main'); expect(api.runs).toHaveBeenCalledWith('origin', 'feature/test');
	expect(watcher.subscribe).toHaveBeenLastCalledWith(expect.any(Function), 5000); expect(api.log).not.toHaveBeenCalled(); expect(pipelines.liveJob?.steps[0].state).toBe('in_progress');
});
it('completed GitHub jobs and partial GitLab jobs read text logs', async () => {
	await pipelines.load('/repo'); await pipelines.setView('live'); vi.mocked(api.jobs).mockResolvedValue({ data: { jobs: [{ ...liveJob, state: 'completed', result: 'success' }], page: 1, more: false, etag: null }, cacheWarning: null });
	await pipelines.readJobs(); expect(pipelines.log).toBe('build output');
	vi.mocked(api.remotes).mockResolvedValue([{ ...host, repo: { ...host.repo, kind: 'gitLab', host: 'gitlab.com' } }]); vi.mocked(api.jobs).mockResolvedValue({ data: { jobs: [liveJob], page: 1, more: false, etag: null }, cacheWarning: null });
	await pipelines.load('/gitlab'); expect(api.log).toHaveBeenCalledWith('origin', 5); expect(pipelines.log).toBe('build output');
});
it('cancel is sent once after confirmation and an uncertain write stays visible', async () => {
	await pipelines.load('/repo'); await pipelines.setView('live'); vi.mocked(api.write).mockRejectedValueOnce('No answer; it may have succeeded.');
	const action = pipelines.send('cancel'); expect(api.write).not.toHaveBeenCalled(); expect(dialog.question?.body).toContain('github.com/team/app'); dialog.accept(); expect(await action).toBe(false); expect(api.write).toHaveBeenCalledOnce(); expect(pipelines.errors.write).toContain('may have succeeded');
	await pipelines.refresh(); expect(pipelines.errors.write).toContain('may have succeeded');
});
it('dismissing rerun and lint confirmations makes no host write', async () => {
	await pipelines.load('/repo'); await pipelines.setView('live'); const rerun = pipelines.send('rerunFailed'); dialog.dismiss(); expect(await rerun).toBe(false);
	const lint = pipelines.send('lint'); dialog.dismiss(); expect(await lint).toBe(false); expect(api.write).not.toHaveBeenCalled();
});
it('watching is saved per remote and a stale log cannot replace the newly selected job', async () => {
	await pipelines.load('/repo'); await pipelines.setView('live'); vi.mocked(api.preferences).mockResolvedValue({ branches: ['release/3.1'] }); await pipelines.watch('release/3.1'); expect(api.preferences).toHaveBeenLastCalledWith('origin', ['release/3.1']);
	vi.mocked(api.jobs).mockResolvedValue({ data: { jobs: [{ ...liveJob, state: 'completed' }, { ...liveJob, id: 6, state: 'completed' }], page: 1, more: false, etag: null }, cacheWarning: null }); await pipelines.readJobs();
	let finish!: (text: string) => void; vi.mocked(api.log).mockReturnValueOnce(new Promise((done) => { finish = done; })); const old = pipelines.selectLiveJob(5); await pipelines.selectLiveJob(6); finish('old output'); await old; expect(pipelines.log).toBe('build output');
});
it('mounted preview selects jobs, opens the exact source and shows matrix details', async () => {
	await pipelines.load('/repo'); vi.mocked(api.source).mockResolvedValue('on: push\njobs:\n  build: {}'); const ui = render(PipelinesScreen, {});
	click(ui.get('[aria-label="Select test"]')); expect(ui.all('dd')[1].textContent).toBe('build'); click(ui.get('.source-link')); await settle(); expect(api.source).toHaveBeenCalledWith(workflow.file); expect(ui.text()).toContain('Requested line 8'); click(ui.get('[aria-label="Select build"]')); expect(ui.text()).toContain('2 combinations'); ui.destroy();
});
it('matrix expansion and both layouts share dependency edges without drawing every offscreen card', () => {
	const select = vi.fn(); const ui = render(PipelineGraph, { jobs: workflow.jobs, onselect: select }); expect(ui.all('path')).toHaveLength(1);
	click(ui.get('[aria-label="Expand build matrix"]')); expect(ui.all('[aria-label="Select build"]')).toHaveLength(2); expect(ui.all('path')).toHaveLength(2); click(ui.get('[aria-label="Select test"]')); expect(select).toHaveBeenCalledWith('test'); ui.destroy();
	const arranged = layout(workflow.jobs, [], 'needs', new Set(['build']), false); expect(arranged.cards[2].x).toBeGreaterThan(arranged.cards[0].x);
});
it('large workflows start collapsed and ambiguous host names get no invented dependency', async () => {
	const rows = Array.from({ length: 201 }, (_, i) => job(`job${i}`, { stage: 'test' })); const ui = render(PipelineGraph, { jobs: rows, stages: ['test'], onselect: vi.fn() }); expect(ui.text()).toContain('Expand stages'); expect(ui.all('.card')).toHaveLength(1); ui.destroy();
	const mapped = liveGraph([{ ...liveJob, id: 1, name: 'build (linux)' }, { ...liveJob, id: 2, name: 'build (windows)' }, liveJob], workflow); expect(mapped[2].needs).toEqual(['1', '2']);
	const ambiguous = liveGraph([liveJob], { ...workflow, jobs: [job('a', { name: 'test' }), job('b', { name: 'test' })] }); expect(ambiguous[0].needs).toEqual([]);
});
it('refresh retains a selected job on a later page and inactive branches use sixty seconds', async () => {
	await pipelines.load('/repo'); await pipelines.setView('live');
	vi.mocked(api.jobs).mockImplementation(async (_, __, page) => ({ data: { jobs: [{ ...liveJob, id: page === 1 ? 5 : 6, state: 'completed' }], page, more: page === 1, etag: null }, cacheWarning: null }));
	await pipelines.readJobs(); await pipelines.readJobs(true); await pipelines.selectLiveJob(6); await pipelines.readJobs(); expect(pipelines.jobs).toHaveLength(2); expect(pipelines.liveJob?.id).toBe(6);
	vi.mocked(api.runs).mockImplementation(async (_, branch) => ({ data: { runs: [{ ...run, branch, state: 'completed', result: 'success' }], etag: null, more: false }, cacheWarning: null }));
	await pipelines.refresh(); expect(watcher.subscribe).toHaveBeenLastCalledWith(expect.any(Function), 60_000);
	vi.mocked(api.runs).mockClear(); await pipelines.refresh(false); expect(api.runs).not.toHaveBeenCalled();
});
