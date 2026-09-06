// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { click, fire, flushSync, render, type Mounted } from '../../testing/mount';
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/rebase/actions');
import * as api from '$lib/api';
import { goto } from '$app/navigation';
import * as actions from '$lib/rebase/actions';
import { rebase } from '$lib/rebase/store.svelte';
import Page from './+page.svelte';
let view: Mounted;
const button = (name: string) => view.all('button').find(b => b.textContent?.trim() === name)!;
const row = { id: 'a'.repeat(40), short: 'aaaaaaa', summary: 'Keep this work', authorName: 'Ada', time: 100, paths: ['a.rs'] };
beforeEach(() => {
 vi.clearAllMocks(); rebase.clear();
 vi.mocked(api.rebaseTodo).mockResolvedValue({ upstream: 'b'.repeat(40), upstreamShort: 'bbbbbbb', rows: [row], truncated: false });
 vi.mocked(api.rebasePreview).mockResolvedValue({ rows: [], dropped: [], refusal: null, emptiesTheBranch: false });
});
afterEach(() => { view?.destroy(); vi.restoreAllMocks(); });
describe('rebase route', () => {
 it('requires an upstream, previews a plan, and passes its scope to Apply', async () => {
  view = render(Page, {});
  expect((button('Plan') as HTMLButtonElement).disabled).toBe(true);
  const input = view.get('input') as HTMLInputElement; input.value = 'main'; fire(input, 'input');
  click(button('Plan')); await vi.waitFor(() => expect(rebase.loaded).toBe(true)); flushSync();
  expect(api.rebaseTodo).toHaveBeenCalledWith('main'); expect(view.text()).toContain('Keep this work');
  expect((button('Apply') as HTMLButtonElement).disabled).toBe(false);
  click(button('Apply')); expect(actions.runRebase).toHaveBeenCalledWith('', 1, 0);
  await rebase.setAction(row.id, 'drop'); flushSync(); click(button('Reset'));
  expect(rebase.plan[0].action).toBe('pick');
 });
 it('shows the read failure, then recovers to an empty plan', async () => {
  vi.mocked(api.rebaseTodo).mockRejectedValueOnce(new Error('unknown revision'));
  rebase.upstream = 'missing'; view = render(Page, {}); click(button('Plan'));
  await vi.waitFor(() => expect(view.text()).toContain('unknown revision'));
  vi.mocked(api.rebaseTodo).mockResolvedValue({ upstream: 'b', upstreamShort: 'b', rows: [], truncated: false });
  click(button('Plan')); await vi.waitFor(() => expect(view.text()).toContain('nothing to rebase'));
  expect((button('Apply') as HTMLButtonElement).disabled).toBe(true);
 });
 it('disables Apply when the preview refuses a rewrite', async () => {
  vi.mocked(api.rebasePreview).mockResolvedValue({ rows: [], dropped: [], refusal: 'Cannot squash the first commit', emptiesTheBranch: false });
  rebase.upstream = 'main'; await rebase.load(); view = render(Page, {});
  expect((button('Apply') as HTMLButtonElement).disabled).toBe(true);
  expect(button('Apply').title).toBe('Cannot squash the first commit');
 });
 it.each([null, { step: 2, total: 3, branch: 'topic' }])('shows running progress and disables history editing (%j)', progress => {
  vi.spyOn(rebase, 'running', 'get').mockReturnValue(true);
  vi.spyOn(rebase, 'progress', 'get').mockReturnValue(progress as never);
  view = render(Page, {});
  expect((button('Plan') as HTMLButtonElement).disabled).toBe(true);
  expect((button('Apply') as HTMLButtonElement).disabled).toBe(true);
  expect(view.get('[role="progressbar"]').getAttribute('aria-valuenow')).toBe(String(progress?.step ?? 0));
  expect(view.text()).toContain(progress ? 'Replaying commit 2 of 3' : 'Starting the rebase');
 });
 it('offers explicit conflict recovery actions when Git stops', () => {
  vi.spyOn(rebase, 'stopped', 'get').mockReturnValue(true);
  vi.spyOn(rebase, 'progress', 'get').mockReturnValue({ step: 1, total: 3, branch: 'topic' } as never);
  vi.spyOn(rebase, 'runError', 'get').mockReturnValue('Resolve a.rs');
  view = render(Page, {}); expect(view.text()).toContain('Resolve a.rs');
  click(button('Resolve conflicts')); expect(goto).toHaveBeenCalledWith('/conflicts');
  click(button('Continue')); expect(actions.continueRebase).toHaveBeenCalled();
  click(button('skip this commit')); expect(actions.skipCommit).toHaveBeenCalled();
  click(button('abort')); expect(actions.abortRebase).toHaveBeenCalled();
  expect((button('Apply') as HTMLButtonElement).disabled).toBe(true);
 });
 it('reports a truncated plan instead of claiming the entire history is shown', async () => {
  vi.mocked(api.rebaseTodo).mockResolvedValue({ upstream: 'b', upstreamShort: 'b', rows: [row], truncated: true });
  rebase.upstream = 'main'; await rebase.load(); view = render(Page, {});
  expect(view.text()).toContain('Only the first 1 commits are shown');
 });
});
