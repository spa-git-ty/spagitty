// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, press, render, type Mounted } from '../../testing/mount';
import { sampleSnapshot, sampleTask } from '../../testing/farm-fixtures';
import { control, calls } from '../../testing/repo-store.svelte';
import type { FarmSnapshot, Task, TaskDetail, AgentRun } from '$lib/farm/types';
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/farm') } }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/farm/api');
vi.mock('$lib/farm/delight');
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }));
vi.mock('$lib/ui/dialog.svelte', () => ({
	dialog: { confirm: vi.fn(async () => true), prompt: vi.fn(async () => 'Use the public CA.') }
}));
vi.mock('$lib/ui/notice.svelte', () => ({ notice: { ok: vi.fn(), failed: vi.fn() } }));
import * as api from '$lib/farm/api';
import { page } from '$app/state';
import { goto } from '$app/navigation';
import { dialog } from '$lib/ui/dialog.svelte';
import { notice } from '$lib/ui/notice.svelte';
import { farmStore } from '$lib/farm/store.svelte';
import Page from './+page.svelte';
let view: Mounted;
let snapshot: FarmSnapshot;
let evidence: Record<string, Partial<TaskDetail>>;
const button = (name: string) => {
	const b = view.all('button').find((b) => b.textContent?.replace(/\s+/g, ' ').trim() === name);
	if (!b) throw Error(`Missing ${name}: ${view.text()}`);
	return b;
};
function type(field: HTMLElement, value: string) {
	(field as HTMLInputElement).value = value;
	fire(field, 'input');
}
async function show(tasks: Task[] = [], query = '', change?: (s: FarmSnapshot) => void) {
	snapshot = sampleSnapshot(tasks);
	change?.(snapshot);
	(page as { url: URL }).url = new URL(`http://localhost/farm${query}`);
	view = render(Page, {});
	await vi.waitFor(() => expect(farmStore.loaded).toBe(true));
	await farmStore.loadDetails();
	flushSync();
}
function run(task = 'T1', phase: AgentRun['phase'] = 'implementation'): AgentRun {
	return {
		id: 'R1',
		task,
		agent: 'claude-1',
		phase,
		command: ['claude'],
		outcome: { state: 'running' },
		startedMs: Date.now() - 240000,
		lastOutputMs: Date.now() - 230000,
		endedMs: null,
		logFile: null
	};
}
beforeEach(() => {
	vi.clearAllMocks();
	farmStore.reset();
	control.reset();
	evidence = {};
	control.setInfo({
		path: '/test/repo',
		name: 'repo',
		bare: false,
		head: { id: 'a', short: 'a', branch: 'main', unborn: false },
		lastFetched: null
	} as never);
	vi.mocked(api.open).mockImplementation(async () => snapshot);
	vi.mocked(api.snapshot).mockImplementation(async () => snapshot);
	vi.mocked(api.stale).mockResolvedValue([]);
	vi.mocked(api.taskDetail).mockImplementation(async (id) => ({
		task: snapshot.farm!.tasks.find((t) => t.id === id)!,
		verification: null,
		review: null,
		handoff: null,
		runs: [],
		...evidence[id]
	}));
	vi.mocked(api.failure).mockImplementation((cause) => ({ kind: 'test', message: String(cause) }));
	vi.mocked(dialog.confirm).mockResolvedValue(true);
});
afterEach(async () => {
	view?.destroy();
	await farmStore.stop();
	farmStore.reset();
	control.reset();
});
describe('Farm journey routes and actions', () => {
	it('opens a repository from the empty screen', () => {
		control.setInfo(null);
		view = render(Page, {});
		click(button('Open repository…'));
		expect(calls.chosen).toBe(1);
		expect(api.open).not.toHaveBeenCalled();
	});
	it('renders the five phases and a six-step track on each board card', async () => {
		await show([sampleTask('T1')]);
		expect(view.all('.farm-phases li')).toHaveLength(5);
		expect(view.all('.task-track span')).toHaveLength(6);
		expect(view.get('[aria-current="step"]').textContent).toContain('Build');
	});
	it('uses task deep links without a page reload', async () => {
		await show([sampleTask('T1')]);
		click(view.get('.board .click-card'));
		expect(goto).toHaveBeenCalledWith('/farm?task=T1');
	});
	it('keeps dependency-blocked tasks Up next and failures in Needs you', async () => {
		await show([
			sampleTask('T1'),
			sampleTask('T2', { status: 'blocked', dependsOn: ['T1'] }),
			sampleTask('T3', { status: 'failed' })
		]);
		expect(view.all('.attention-grid article')).toHaveLength(1);
		expect(view.get('.attention-grid').textContent).toContain('T3');
		expect(view.get('.board').textContent).toContain('T2');
	});
	it('marks a quiet live run after three minutes', async () => {
		await show([sampleTask('T1', { status: 'running' })], '', (s) => (s.runs = [run()]));
		expect(view.text()).toContain('quiet for 3 min');
		expect(view.all('.agent-badge.running').length).toBeGreaterThan(0);
	});
	it('does not show a finished agent as working', async () => {
		await show(
			[sampleTask('T1')],
			'',
			(s) =>
				(s.runs = [{ ...run(), outcome: { state: 'completed', exitCode: 0 }, endedMs: Date.now() }])
		);
		expect(view.all('.agent-badge.running')).toHaveLength(0);
	});
	it('lands only checked and approved work from Needs you', async () => {
		evidence.T1 = {
			verification: { passed: true, unverified: false, results: [] },
			review: { decision: 'approve', summary: 'Good', issues: [] }
		};
		await show([sampleTask('T1', { status: 'review' })]);
		click(button('Land it'));
		await vi.waitFor(() => expect(api.mergeTask).toHaveBeenCalledWith('T1'));
	});
	it('does not present unverified work as ready to land', async () => {
		evidence.T1 = {
			verification: { passed: true, unverified: true, results: [] },
			review: { decision: 'approve', summary: 'Good', issues: [] }
		};
		await show([sampleTask('T1', { status: 'review' })]);
		expect(view.all('button').some((b) => b.textContent === 'Land it')).toBe(false);
	});
	it('pauses without cancelling the agents', async () => {
		await show([sampleTask('T1', { status: 'running' })]);
		click(button('Pause'));
		await vi.waitFor(() => expect(api.pause).toHaveBeenCalledOnce());
		expect(api.cancel).not.toHaveBeenCalled();
		expect(api.cancelTask).not.toHaveBeenCalled();
	});
	it('resumes a paused farm and explains pause accurately', async () => {
		await show([sampleTask('T1')], '', (s) => (s.farm!.status = 'paused'));
		expect(view.text()).toContain('agents already working are left to finish');
		click(button('Resume'));
		await vi.waitFor(() => expect(api.start).toHaveBeenCalledOnce());
	});
	it('expands container children in place', async () => {
		await show([sampleTask('P'), sampleTask('C', { parent: 'P' })]);
		expect(view.all('.click-card')).toHaveLength(1);
		click(view.get('.click-card'));
		expect(view.all('.click-card')).toHaveLength(2);
	});
	it('hides cancelled work until requested', async () => {
		await show([sampleTask('T1'), sampleTask('T2', { status: 'cancelled' })]);
		expect(view.all('.click-card')).toHaveLength(1);
		click(view.get('input[type="checkbox"]'));
		expect(view.all('.click-card')).toHaveLength(2);
	});
	it('saves rules as one action and keeps a rejected form open', async () => {
		await show([sampleTask('T1')]);
		click(button('Rules'));
		click(button('Add a command'));
		type(view.get('[aria-label="Check 2"]'), 'bun run check');
		vi.mocked(api.configure).mockRejectedValueOnce('refused');
		click(button('Save rules'));
		await vi.waitFor(() => expect(notice.failed).toHaveBeenCalled());
		expect(api.configure).toHaveBeenCalledWith(
			expect.objectContaining({ verification: ['cargo test', 'bun run check'] })
		);
		expect(view.find('[role="dialog"]')).not.toBeNull();
		expect((button('Save rules') as HTMLButtonElement).disabled).toBe(false);
	});
	it('creates tasks with the existing editor', async () => {
		await show();
		click(button('Task'));
		type(view.get('[role="dialog"] input'), '  Test the farm  ');
		click(button('Add task'));
		await vi.waitFor(() =>
			expect(api.addTask).toHaveBeenCalledWith(expect.objectContaining({ title: 'Test the farm' }))
		);
	});
	it('creates, configures and then plans from Setup', async () => {
		await show([], '', (s) => (s.farm = null));
		type(view.get('.setup input:not([type="checkbox"])'), '  Ship the goal  ');
		click(button('Plan it with Claude Code'));
		await vi.waitFor(() => expect(api.plan).toHaveBeenCalledWith('claude-1'));
		expect(api.create).toHaveBeenCalledWith('Ship the goal', '');
		expect(vi.mocked(api.create).mock.invocationCallOrder[0]).toBeLessThan(
			vi.mocked(api.configure).mock.invocationCallOrder[0]
		);
		expect(vi.mocked(api.configure).mock.invocationCallOrder[0]).toBeLessThan(
			vi.mocked(api.plan).mock.invocationCallOrder[0]
		);
	});
	it('plans on Enter in the goal field, and not while the goal is empty', async () => {
		await show([], '', (s) => (s.farm = null));
		const goal = view.get('input.goal');
		press(goal, 'Enter');
		expect(api.create).not.toHaveBeenCalled();
		type(goal, 'Ship it');
		press(goal, 'Enter');
		await vi.waitFor(() => expect(api.plan).toHaveBeenCalledWith('claude-1'));
		expect(api.create).toHaveBeenCalledWith('Ship it', '');
	});

	it('refuses to start a farm with no goal', async () => {
		await show([], '', (s) => (s.farm = null));
		expect((button("I'll write the tasks myself") as HTMLButtonElement).disabled).toBe(true);
		expect((button('Plan it with Claude Code') as HTMLButtonElement).disabled).toBe(true);
	});

	it('names the agents it did not find in one quiet line', async () => {
		await show([], '', (s) => (s.farm = null));
		expect(view.text()).toContain('Not found: Cursor, Oh My Pi.');
		expect(view.all('.tile')).toHaveLength(2);
	});

	it('offers to write AGENTS.md only when there is none', async () => {
		await show([], '', (s) => (s.farm = null));
		expect(view.text()).toContain('AGENTS.md is attached to every prompt');
		expect(view.text()).not.toContain('Write the starter AGENTS.md');
		view.destroy();
		farmStore.reset();
		await show([], '', (s) => {
			s.farm = null;
			s.policy = { sources: [], text: '' };
		});
		click(button('Write the starter AGENTS.md'));
		await vi.waitFor(() => expect(api.writePolicy).toHaveBeenCalledOnce());
	});

	it('grants the merge permission with a level that merges by itself', async () => {
		await show([sampleTask('T1')]);
		click(button('Rules'));
		click(view.all('[role="radio"]').find((b) => b.textContent?.includes('Automatic'))!);
		click(button('Save rules'));
		await vi.waitFor(() =>
			expect(api.configure).toHaveBeenCalledWith(
				expect.objectContaining({
					autonomy: 'auto',
					permissions: expect.objectContaining({ merge: true })
				})
			)
		);
	});

	it('says plainly when no check runs on the plan', async () => {
		await show([sampleTask('A', { status: 'draft' })], '', (s) => (s.farm!.verification = []));
		expect(view.text()).toContain('No checks: nothing proves a task works');
	});

	it('keeps planning disabled with no eligible planner', async () => {
		await show([], '', (s) => {
			s.farm = null;
			s.agents.forEach((a) => (a.definition.enabled = false));
		});
		type(view.get('.setup input:not([type="checkbox"])'), 'Goal');
		expect((view.all('.setup button').at(-1) as HTMLButtonElement).disabled).toBe(true);
	});
	it('shows planning output and stops only planning', async () => {
		await show([], '', (s) => (s.runs = [run('planning', 'planning')]));
		farmStore.absorb({
			kind: 'agentOutput',
			run: 'R1',
			task: 'planning',
			line: 'Reading the repository',
			atMs: Date.now()
		});
		flushSync();
		expect(view.text()).toContain('Reading the repository');
		click(button('Stop planning'));
		await vi.waitFor(() => expect(api.cancelPlan).toHaveBeenCalledOnce());
		expect(api.cancel).not.toHaveBeenCalled();
	});
	it('accepts the kept tasks, discards the rest, then starts', async () => {
		await show([sampleTask('A', { status: 'draft' }), sampleTask('B', { status: 'draft' })]);
		click(view.get('[aria-label="Keep B"]'));
		click(button('Start building · 1 task'));
		await vi.waitFor(() => expect(api.start).toHaveBeenCalledOnce());
		expect(api.readyTasks).toHaveBeenCalledWith(['A']);
		expect(api.discardTasks).toHaveBeenCalledWith(['B']);
		expect(vi.mocked(api.readyTasks).mock.invocationCallOrder[0]).toBeLessThan(
			vi.mocked(api.discardTasks).mock.invocationCallOrder[0]
		);
	});
	it('prevents accepting a plan with an omitted prerequisite', async () => {
		await show([
			sampleTask('A', { status: 'draft' }),
			sampleTask('B', { status: 'draft', dependsOn: ['A'] })
		]);
		click(view.get('[aria-label="Keep A"]'));
		expect((button('Start building · 1 task') as HTMLButtonElement).disabled).toBe(true);
		expect(view.text()).toContain('needs A, left out');
	});
	it('never starts after accepting the plan fails', async () => {
		await show([sampleTask('A', { status: 'draft' })]);
		vi.mocked(api.readyTasks).mockRejectedValueOnce('refused');
		click(button('Start building · 1 task'));
		await vi.waitFor(() => expect(notice.failed).toHaveBeenCalled());
		expect(api.start).not.toHaveBeenCalled();
		expect(api.discardTasks).not.toHaveBeenCalled();
	});
	it('routes a task to its full stepper and stops its run', async () => {
		await show([sampleTask('T1', { status: 'running' })], '?task=T1', (s) => (s.runs = [run()]));
		expect(view.all('.stepper li')).toHaveLength(6);
		click(button('Stop'));
		await vi.waitFor(() => expect(api.cancelTask).toHaveBeenCalledWith('T1'));
	});
	it('shows a failed task Checks tab and retries with an available agent', async () => {
		evidence.T1 = {
			verification: {
				passed: false,
				unverified: false,
				results: [
					{ command: 'cargo test', passed: false, output: 'missing library', durationMs: 1000 }
				]
			}
		};
		await show([sampleTask('T1', { status: 'failed', attempts: 3 })], '?task=T1');
		expect(view.text()).toContain('missing library');
		click(button('Retry with Claude Code'));
		await vi.waitFor(() => expect(api.runTask).toHaveBeenCalledWith('T1', 'claude-1'));
	});
	it('opens a ready task on its review, and keeps a tab the person picked', async () => {
		evidence.T1 = {
			verification: { passed: true, unverified: false, results: [] },
			review: { decision: 'approve', summary: 'Clean split.', issues: [] }
		};
		await show([sampleTask('T1', { status: 'review' })], '?task=T1');
		expect(view.text()).toContain('Clean split.');
		click(button('Output'));
		expect(view.text()).not.toContain('Clean split.');
	});

	it('shows real change statistics', async () => {
		evidence.T1 = {
			stats: { commits: 2, files: [{ path: 'src/a.ts', added: 4, removed: 2, binary: false }] }
		};
		await show([sampleTask('T1')], '?task=T1&tab=Changes');
		expect(view.text()).toContain('src/a.ts');
		expect(view.text()).toContain('+4');
		expect(view.text()).toContain('−2');
	});
	it('confirms deleting a task', async () => {
		await show([sampleTask('T1')], '?task=T1');
		vi.mocked(dialog.confirm).mockResolvedValueOnce(false);
		click(button('Delete'));
		await vi.waitFor(() => expect(dialog.confirm).toHaveBeenCalled());
		expect(api.deleteTask).not.toHaveBeenCalled();
	});
	it('shows crew detection, enabled switches and arguments', async () => {
		await show([sampleTask('T1')], '?pane=crew');
		click(button('Look again'));
		await vi.waitFor(() => expect(api.detectAgents).toHaveBeenCalledOnce());
		expect(view.text()).toContain('Claude Code');
		expect(view.text()).toContain('Not found: Cursor');
	});
	it('opens the custom CLI form and saves its definition', async () => {
		await show([], '?pane=crew');
		click(button('Add a CLI agent'));
		const fields = view.all('[role="dialog"] input');
		type(fields[0], 'My CLI');
		type(fields[1], 'C:/tools/agent.exe');
		click(button('Save'));
		await vi.waitFor(() =>
			expect(api.saveAgent).toHaveBeenCalledWith(
				expect.objectContaining({
					provider: 'custom',
					displayName: 'My CLI',
					executable: 'C:/tools/agent.exe'
				})
			)
		);
	});
	it('filters Activity failures and links their tasks', async () => {
		await show(
			[sampleTask('T1')],
			'?pane=activity',
			(s) =>
				(s.events = [
					{
						kind: 'verificationFinished',
						task: 'T1',
						command: 'test',
						passed: false,
						output: 'oops',
						atMs: Date.now() - 10000
					},
					{ kind: 'taskCreated', task: 'T1', title: 'Task', atMs: Date.now() }
				])
		);
		click(button('Failures'));
		expect(view.all('.event')).toHaveLength(1);
		click(button('T1'));
		expect(goto).toHaveBeenCalledWith('/farm?task=T1');
	});
	it('shows wrap-up hashes and reads every task hand-off', async () => {
		evidence.T1 = {
			mergeSha: 'abcdef123',
			handoff: {
				status: 'completed',
				summary: 'Done',
				filesChanged: [],
				commits: [],
				tests: [],
				risks: ['Needs follow-up'],
				questions: [],
				proposedTasks: []
			}
		};
		await show(
			[sampleTask('T1', { status: 'done', attempts: 1 })],
			'',
			(s) => (s.farm!.status = 'completed')
		);
		expect(view.text()).toContain('has landed');
		expect(view.text()).toContain('abcdef1');
		expect(view.text()).toContain('Needs follow-up');
		click(button('Start a new goal'));
		expect(view.text()).toContain('Put a crew of agents on one goal');
	});
	it('reports a refused merge and restores controls', async () => {
		evidence.T1 = {
			verification: { passed: true, unverified: false, results: [] },
			review: { decision: 'approve', summary: 'Good', issues: [] }
		};
		await show([sampleTask('T1', { status: 'review' })]);
		vi.mocked(api.mergeTask).mockRejectedValueOnce('Switch back to main');
		click(button('Land it'));
		await vi.waitFor(() => expect(notice.failed).toHaveBeenCalled());
		expect((button('Land it') as HTMLButtonElement).disabled).toBe(false);
	});
});
