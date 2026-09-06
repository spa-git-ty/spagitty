// SPDX-License-Identifier: GPL-3.0-or-later
import { beforeEach, afterEach, expect, it, vi } from 'vitest';
import { click, fire, render, type Mounted } from '../../testing/mount';
import { control } from '../../testing/repo-store.svelte';
import { branchRow } from '../../testing/branches-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$lib/branches/actions', async importOriginal => ({ ...await importOriginal<object>(), deleteMerged: vi.fn() }));
vi.mock('$lib/network/store.svelte', () => ({ network: { running: false, fetch: vi.fn() } }));
import * as api from '$lib/api';
import { branches } from '$lib/branches/store.svelte';
import { deleteMerged } from '$lib/branches/actions';
import { network } from '$lib/network/store.svelte';
import Page from './+page.svelte';
let view: Mounted;
const button = (name: string) => view.all('button').find(b => b.textContent?.trim() === name)!;
const type = (selector: string, value: string) => { const input = view.get(selector) as HTMLInputElement; input.value = value; fire(input, 'input'); };
beforeEach(() => {
 vi.clearAllMocks(); branches.clear(); control.reset();
 control.setInfo({ path: '/test', name:'test', bare:false, head: { branch: 'main' }, lastFetched:null } as never);
 vi.mocked(api.branches).mockResolvedValue([branchRow({current:true, upstream:'origin/main'}), branchRow({name:'old', fullName:'refs/heads/old', merged:true}), branchRow({name:'origin/main', fullName:'refs/remotes/origin/main',kind:'remote'})]);
});
afterEach(() => { view?.destroy(); branches.clear(); control.reset(); });
it('filters branches, explains stale drift, and sends refresh/cleanup to the right action', async () => {
 view = render(Page, {}); await vi.waitFor(() => expect(branches.loaded).toBe(true));
 expect(view.text()).toContain('2 local · 1 remote-tracking'); expect(view.text()).toContain('never fetched');
 type('[aria-label="Filter branches by name"]','old'); expect(view.text()).toContain('2 hidden');
 click(button('clear')); expect(branches.hidden).toBe(0);
 click(button('mine')); expect(branches.active).toEqual(['mine']); click(button('mine'));
 click(button('Fetch')); expect(network.fetch).toHaveBeenCalled();
 click(button('Delete merged')); expect(deleteMerged).toHaveBeenCalledWith(branches.rows);
 click(button('Refresh')); await vi.waitFor(() => expect(api.branches).toHaveBeenCalledTimes(2));
});
it('creates a branch using the entered start point and checkout choice', async () => {
 view = render(Page, {}); await vi.waitFor(() => expect(branches.loaded).toBe(true));
 expect((button('Create') as HTMLButtonElement).disabled).toBe(true);
 type('[aria-label="New branch name"]', 'feature/safe'); type('[aria-label="Start point for the new branch"]','main');
 click(button('check it out')); click(button('Create'));
 await vi.waitFor(() => expect(api.createBranch).toHaveBeenCalled());
 expect(api.createBranch).toHaveBeenCalledWith('feature/safe','main',false);
});
it('shows read failures and recovers on refresh', async () => {
 vi.mocked(api.branches).mockRejectedValueOnce(new Error('read failed'));
 view=render(Page,{}); await vi.waitFor(() => expect(view.text()).toContain('read failed'));
 click(button('Refresh')); await vi.waitFor(() => expect(branches.loaded).toBe(true));
 expect(view.text()).not.toContain('read failed');
});
it('explains that a repository is needed without issuing a read', () => {
 control.setInfo(null); view=render(Page,{}); expect(view.text()).toContain('No repository open'); expect(api.branches).not.toHaveBeenCalled();
});
