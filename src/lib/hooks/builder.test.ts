// SPDX-License-Identifier: GPL-3.0-or-later

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render } from '../../testing/mount';
import { makeBlock, steps, type Builder, type Run } from './builder-types';

const listeners: (() => void)[] = [];
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async (_name: string, callback: () => void) => { listeners.push(callback); return vi.fn(); }) }));
vi.mock('$app/navigation', () => ({ goto: vi.fn(async () => {}) }));
vi.mock('./builder-api', () => ({ load: vi.fn(), save: vi.fn(async () => {}), importManager: vi.fn(), matches: vi.fn(async () => ['a.txt']), runs: vi.fn(async () => []), run: vi.fn(), log: vi.fn(async () => 'checking\n'), dryRun: vi.fn(async () => ({ passed: true, output: '' })), tryMessage: vi.fn(async () => ({ passed: false, output: 'The subject does not match.' })), stop: vi.fn(async () => {}), undoRestage: vi.fn(async () => {}) }));

import * as api from './builder-api';
import { builder } from './builder.svelte';
import { dialog } from '$lib/ui/dialog.svelte';
import HooksScreen from './HooksScreen.svelte';
import HookRuns from './HookRuns.svelte';

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
const definition = (): Builder => ({ definition: { version: 1, shared: false, activeStages: ['pre-commit', 'commit-msg', 'pre-push'], blocks: [] }, info: { dir: '.git/hooks', manager: 'git', config: null, hooks: [], enabled: true }, runs: [], staged: ['a.txt'], installed: false });
const record = (id = 'run-1'): Run => ({ id, stage: 'pre-commit', subject: 'feat: refund', at: '2026-10-14T10:00:00Z', exit: 0, events: [
	{ t: 'run', blocks: [{ id: 'tests', label: 'Tests · payments' }, { id: 'script', label: 'Script' }] },
	{ t: 'start', id: 'tests', label: 'Tests · payments' }, { t: 'match', id: 'tests', files: 3 },
	{ t: 'end', id: 'tests', exit: 0, ms: 2000 }, { t: 'skip', id: 'script', why: 'no-match' }, { t: 'done', exit: 0 }
] });

beforeEach(() => {
	vi.clearAllMocks(); builder.clear(); dialog.dismiss(); listeners.length = 0;
	vi.mocked(api.load).mockImplementation(async () => definition());
	vi.mocked(api.runs).mockResolvedValue([]);
	vi.mocked(api.run).mockImplementation(async (id) => record(id));
	vi.mocked(api.log).mockResolvedValue('checking\n');
	vi.mocked(api.save).mockResolvedValue();
});

describe('hook builder state', () => {
	it('keeps an unsaved draft when switching between its views', async () => {
		await builder.load('/repo'); builder.add('tests'); builder.share(true);
		await builder.load('/repo');
		expect(builder.draft?.blocks).toHaveLength(1); expect(builder.draft?.shared).toBe(true);
		expect(api.load).toHaveBeenCalledTimes(1); expect(builder.dirty).toBe(true);
	});
	it('drops a slow read from the repository that was left', async () => {
		let answer!: (value: Builder) => void;
		vi.mocked(api.load).mockImplementationOnce(() => new Promise((resolve) => { answer = resolve; }));
		const old = builder.load('/old'); await settle();
		await builder.load('/new'); builder.add('secrets'); answer(definition()); await old;
		expect(builder.draft?.blocks[0].action.kind).toBe('secrets');
	});
	it('reorders only within a stage and removes the selected block', async () => {
		await builder.load('/repo'); builder.add('tests'); builder.add('script'); builder.add('message');
		const tests = builder.draft!.blocks[0].id; const script = builder.draft!.blocks[1].id;
		builder.move(tests, 1); expect(builder.draft!.blocks[0].id).toBe(script);
		builder.move(tests, 1); expect(builder.draft!.blocks[1].id).toBe(tests);
		builder.remove(builder.block!.id); expect(builder.draft?.blocks).toHaveLength(2);
	});
	it('confirms installing and sends the reviewed snapshot', async () => {
		await builder.load('/repo'); builder.add('tests');
		const saving = builder.save(); expect(api.save).not.toHaveBeenCalled(); dialog.accept(); await saving;
		expect(api.save).toHaveBeenCalledWith(expect.objectContaining({ blocks: [expect.objectContaining({ action: { kind: 'tests', paths: ['**'], command: 'npm test' } })] }), true, false);
		expect(builder.dirty).toBe(false);
	});
	it('does not install or dry-run a cancelled confirmation', async () => {
		await builder.load('/repo'); builder.add('tests');
		const saving = builder.save(); dialog.dismiss(); await saving;
		const running = builder.dryRun('pre-commit'); dialog.dismiss(); await running;
		expect(api.save).not.toHaveBeenCalled(); expect(api.dryRun).not.toHaveBeenCalled();
	});
	it('leaves a manager read-only until an explicit import', async () => {
		const managed = definition(); managed.info.manager = 'lefthook'; vi.mocked(api.load).mockResolvedValue(managed);
		vi.mocked(api.importManager).mockResolvedValue({ ...managed.definition, blocks: [makeBlock('script')] });
		await builder.load('/repo'); builder.add('tests'); expect(builder.draft?.blocks).toHaveLength(0);
		const importing = builder.importManager(); expect(api.importManager).not.toHaveBeenCalled(); dialog.accept(); await importing;
		expect(builder.readonly).toBe(false); expect(builder.imported).toBe(true);
	});
	it('keeps a failed save and a failed log visible after a successful run refresh', async () => {
		await builder.load('/repo'); builder.add('tests'); vi.mocked(api.save).mockRejectedValueOnce('Disk full');
		const saving = builder.save(); dialog.accept(); await saving;
		vi.mocked(api.runs).mockResolvedValue([record()]); vi.mocked(api.log).mockRejectedValueOnce('Log unreadable');
		await builder.refreshRuns(true);
		expect(builder.errors.save).toBe('Disk full'); expect(builder.errors.log).toBe('Log unreadable');
	});
});

describe('the mounted Hooks screens', () => {
	it('adds all seven kinds and edits a command through the real inputs', async () => {
		await builder.load('/repo'); const view = render(HooksScreen, { mode: 'edit' });
		for (const button of view.all('.catalogue-block')) click(button);
		expect(builder.draft?.blocks).toHaveLength(7);
		const script = builder.draft!.blocks.find((block) => block.action.kind === 'script')!; builder.select(script.id); flushSync();
		const command = view.get('[aria-label="Block command"]') as HTMLTextAreaElement;
		command.value = './check.sh'; fire(command, 'input');
		expect(builder.block?.action).toEqual({ kind: 'script', command: './check.sh' });
		click(view.all('button').find((button) => button.textContent?.trim() === 'Remove')!);
		expect(builder.draft?.blocks).toHaveLength(6); view.destroy();
	});
	it('tries a message with the generated-script command instead of a frontend regex', async () => {
		await builder.load('/repo'); builder.add('message'); const view = render(HooksScreen, { mode: 'edit' });
		const field = view.get('[aria-label="Try a message"]') as HTMLTextAreaElement; field.value = 'wrong'; fire(field, 'input');
		click(view.all('button').find((button) => button.textContent?.trim() === 'Try message')!); await settle(); flushSync();
		expect(api.tryMessage).toHaveBeenCalledWith(expect.objectContaining({ blocks: [expect.objectContaining({ stage: 'commit-msg' })] }), 'wrong');
		expect(view.text()).toContain('The subject does not match.'); view.destroy();
	});
	it('selects real steps, shows their log and labels skipped steps', async () => {
		const initial = definition(); initial.runs = [record()]; vi.mocked(api.load).mockResolvedValue(initial);
		await builder.load('/repo'); const view = render(HookRuns, {});
		expect(view.text()).toContain('Tests · payments'); expect(view.text()).toContain('checking'); expect(view.text()).toContain('no change matched');
		click(view.all('.step').find((button) => button.textContent?.includes('Script'))!); await settle();
		expect(api.log).toHaveBeenLastCalledWith('run-1', 'script'); view.destroy();
	});
	it('reads appended records when the repository watcher reports a change', async () => {
		await builder.load('/repo'); vi.mocked(api.runs).mockResolvedValue([record()]);
		listeners[0](); await settle(); await settle(); expect(builder.current?.id).toBe('run-1');
	});
});

it('keeps queued blocks and the distinct warning, skipped and restaged states', () => {
	const run = record(); run.events.push({ t: 'end', id: 'tests', exit: 2, warn: true, ms: 12 }, { t: 'restaged', id: 'tests', files: ['a.txt'] });
	expect(steps(run)).toEqual([expect.objectContaining({ id: 'tests', state: 'warning', matched: 3, restaged: ['a.txt'] }), expect.objectContaining({ id: 'script', state: 'skipped' })]);
});

it('leaves later blocks queued only while the run can still reach them', () => {
	const run = record(); run.events = run.events.slice(0, 2); run.exit = null;
	expect(steps(run)[1].state).toBe('queued');
	run.exit = 2;
	expect(steps(run)[1]).toEqual(expect.objectContaining({ state: 'skipped', why: 'an earlier block stopped the run' }));
});
