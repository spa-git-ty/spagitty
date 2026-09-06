// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render, type Mounted } from '../../testing/mount';
import { sampleSnapshot, sampleTask } from '../../testing/farm-fixtures';
import { control, calls } from '../../testing/repo-store.svelte';
import type { FarmSnapshot, Task, TaskStatus } from '$lib/farm/types';
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/farm') } }));
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/farm/api');
vi.mock('$lib/farm/delight');
vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn(async () => vi.fn()) }));
vi.mock('$lib/ui/dialog.svelte', () => ({ dialog: { confirm: vi.fn(async () => true) } }));
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
const button = (name: string) => {
 const found = view.all('button').find(b => b.textContent?.replace(/\s+/g, ' ').trim() === name);
 if (!found) throw new Error(`Missing ${name}: ${view.text()}`);
 return found;
};
const type = (field: HTMLElement, value: string) => { (field as HTMLInputElement).value = value; fire(field, 'input'); };
async function show(tasks: Task[] = [], pane = 'tasks') {
 snapshot = sampleSnapshot(tasks);
 (page as { url: URL }).url = new URL(`http://localhost/farm?pane=${pane}`);
 view = render(Page, {});
 await vi.waitFor(() => expect(farmStore.farm).not.toBeNull()); flushSync();
}
async function select(task: Task) {
 vi.mocked(api.taskDetail).mockResolvedValue({ task, verification: null, review: null, handoff: null, runs: [] });
 click(view.get('.tasks .row'));
 await vi.waitFor(() => expect(view.find('.detail')).not.toBeNull());
}
beforeEach(() => {
 vi.clearAllMocks(); farmStore.reset(); control.reset();
 control.setInfo({ path: '/test/repo', name: 'repo', bare: false, head: { id: 'a', short: 'a', branch: 'main', unborn: false }, lastFetched: null } as never);
 vi.mocked(api.open).mockImplementation(async () => snapshot);
 vi.mocked(api.snapshot).mockImplementation(async () => snapshot);
 vi.mocked(api.stale).mockResolvedValue([]);
 vi.mocked(api.failure).mockImplementation(cause => ({kind:'test', message: String(cause)}));
 vi.mocked(dialog.confirm).mockResolvedValue(true);
});
afterEach(() => { view?.destroy(); farmStore.reset(); control.reset(); });
describe('farm screen actions', () => {
 it('opens a repository from the empty screen', () => {
  control.setInfo(null); view = render(Page, {}); click(button('Open repository…'));
  expect(calls.chosen).toBe(1); expect(api.open).not.toHaveBeenCalled();
 });
 it('selecting Auto grants merge permission and a lower level revokes it', async () => {
  await show([], 'settings');
  const levels = view.all('.level');
  click(levels[3]); await vi.waitFor(() => expect(api.configure).toHaveBeenCalled());
  expect(api.configure).toHaveBeenLastCalledWith({ autonomy: 'auto', permissions: { ...snapshot.farm!.permissions, merge: true } });
  await vi.waitFor(() => expect((levels[2] as HTMLButtonElement).disabled).toBe(false));
  click(levels[2]); await vi.waitFor(() => expect(api.configure).toHaveBeenCalledTimes(2));
  expect(api.configure).toHaveBeenLastCalledWith({ autonomy: 'semiAuto', permissions: { ...snapshot.farm!.permissions, merge: false } });
 });
 it('saves trimmed goal text, multiline checks and concurrency', async () => {
  await show([], 'settings'); type(view.get('input'), '  Ship safely  ');
  const fields = view.all('textarea'); type(fields[0], '  Keep changes  '); type(fields[1], ' cargo test\n\n bun run check ');
  click(button('Save')); await vi.waitFor(() => expect(api.configure).toHaveBeenCalled());
  expect(api.configure).toHaveBeenCalledWith({ goalTitle: 'Ship safely', goalDescription: 'Keep changes', verification: ['cargo test', 'bun run check'] });
  await vi.waitFor(() => expect((button('8') as HTMLButtonElement).disabled).toBe(false)); click(button('8'));
  await vi.waitFor(() => expect(api.configure).toHaveBeenCalledWith({ maxParallel: 8 }));
 });
 it('accepts only selected planner proposals', async () => {
  await show([sampleTask('T1', {status:'draft'}), sampleTask('T2', {status:'draft'})]);
  click(button('None')); expect((button('Add 0 to the plan') as HTMLButtonElement).disabled).toBe(true);
  click(button('All')); click(view.get('.tasks input[type="checkbox"]'));
  click(button('Add 1 to the plan')); await vi.waitFor(() => expect(api.readyTasks).toHaveBeenCalledWith(['T2']));
 });
 it('requires confirmation before discarding proposals', async () => {
  await show([sampleTask('T1', {status:'draft'})]); vi.mocked(dialog.confirm).mockResolvedValueOnce(false);
  click(button('Discard')); await vi.waitFor(() => expect(dialog.confirm).toHaveBeenCalled()); expect(api.discardTasks).not.toHaveBeenCalled();
  click(button('Discard')); await vi.waitFor(() => expect(api.discardTasks).toHaveBeenCalledWith(['T1']));
 });
 it.each([
  ['ready','Run','runTask'], ['running','Stop','cancelTask'], ['failed','Try again','retryTask'],
  ['review','Verify','verifyTask'], ['review','Send for review','reviewTask'], ['review','Approve and merge','mergeTask'],
  ['draft','Add to the plan','readyTask'], ['ready','Break it down','decompose']
 ] as const)('routes %s / %s to the selected task', async (status, label, method) => {
  const task = sampleTask('T1', { status: status as TaskStatus, worktree: '/test/task', branch: 'task/T1' });
  await show([task]); await select(task); click(button(label));
  await vi.waitFor(() => expect(api[method]).toHaveBeenCalled());
  expect(vi.mocked(api[method]).mock.calls[0]).toEqual(method === 'runTask' || method === 'decompose' ? ['T1', null] : ['T1']);
 });
 it('reports backend failure and releases the busy state', async () => {
  const task = sampleTask('T1'); await show([task]); await select(task);
  vi.mocked(api.runTask).mockRejectedValueOnce(new Error('agent unavailable'));
  click(button('Run')); await vi.waitFor(() => expect(notice.failed).toHaveBeenCalled());
  expect((button('Run') as HTMLButtonElement).disabled).toBe(false);
 });
 it('creates a task from the editor and trims its title', async () => {
  await show([]); click(button('Add task')); type(view.get('.editor input'), '  Add tests  ');
  fire(view.get('form.editor'), 'submit'); await vi.waitFor(() => expect(api.addTask).toHaveBeenCalled());
  expect(vi.mocked(api.addTask).mock.calls[0][0]).toMatchObject({ title: 'Add tests', ready: true, dependsOn: [] });
  await vi.waitFor(() => expect(view.find('.editor')).toBeNull());
 });
 it('deletes only after confirmation and clears selection', async () => {
  const task = sampleTask('T1'); await show([task]); await select(task); click(button('Delete'));
  await vi.waitFor(() => expect(api.deleteTask).toHaveBeenCalledWith('T1'));
  await vi.waitFor(() => expect(view.find('.detail')).toBeNull());
 });
 it('shows detected agent availability and sends a detection request', async () => {
  await show([], 'agents'); expect(view.text()).toContain('Claude Code'); expect(view.text()).toContain('Cursor');
  click(button('Look again'));
  await vi.waitFor(() => expect(api.detectAgents).toHaveBeenCalled());
 });
});
