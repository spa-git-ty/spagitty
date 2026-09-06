// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, fire, press, render, type Mounted } from '../../testing/mount';
import { openRepository, request } from '../../testing/git-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/navigation',()=>({goto:vi.fn()}));
import * as api from '$lib/api';
import { goto } from '$app/navigation';
import { requests } from '$lib/requests/store.svelte';
import { branches } from '$lib/branches/store.svelte';
import Page from './+page.svelte';
let view:Mounted;
const button=(name:string)=>view.all('button').find(b=>b.textContent?.trim()===name)!;
const type=(selector:string,value:string)=>{const input=view.get(selector) as HTMLInputElement;input.value=value;fire(input,'input');};
beforeEach(()=>{
 vi.clearAllMocks();requests.clear();branches.clear();control.reset();openRepository();vi.mocked(api.inTauri).mockReturnValue(true);
 vi.mocked(api.forgeRepo).mockResolvedValue({kind:'gitHub',host:'github.com',owner:'example',name:'demo'});
 vi.mocked(api.pullRequests).mockResolvedValue([request({needsYou:true,needsYouBecause:'Review requested'}),request({id:'PR_2',number:413})]);
 vi.mocked(api.forgeAccounts).mockResolvedValue([]);
});
afterEach(()=>{view?.destroy();requests.clear();control.reset();vi.restoreAllMocks();});
it('separates requested reviews from work waiting on others and opens the selected request',async()=>{
 view=render(Page,{});await vi.waitFor(()=>expect(requests.all).toHaveLength(2));
 expect(view.text()).toContain('1 waiting on you · 1 on others');expect(view.text()).toContain('Review requested');
 const open=vi.spyOn(requests,'openWorkspace').mockResolvedValue();click(view.get('li.row button'));
 expect(open).toHaveBeenCalledWith('PR_1');
});
it('shows the host error and sends account recovery to Settings',async()=>{
 vi.mocked(api.pullRequests).mockRejectedValue(new Error('No account for github.com'));
 view=render(Page,{});await vi.waitFor(()=>expect(view.text()).toContain('No account for github.com'));
 click(button('Settings → Accounts'));expect(goto).toHaveBeenCalledWith('/settings#accounts');
});
it('explains unsupported remotes without requesting pull requests',async()=>{
 vi.mocked(api.forgeRepo).mockResolvedValue(null);view=render(Page,{});
 await vi.waitFor(()=>expect(requests.loading).toBe(false));expect(view.text()).toContain('not on a service');expect(api.pullRequests).not.toHaveBeenCalled();
});
it('distinguishes an empty queue and validates a new pull request before sending',async()=>{
 vi.mocked(api.pullRequests).mockResolvedValue([]);view=render(Page,{});
 await vi.waitFor(()=>expect(requests.connected).toBe(true));expect(view.text()).toContain('Nothing open');
 click(button('+ Create PR'));expect(view.find('[role="dialog"]')).not.toBeNull();
 type('#pr-title',' Fix checks ');type('#head-branch','main');click(button('Create Pull Request'));
 await vi.waitFor(()=>expect(view.text()).toContain('cannot be identical'));expect(api.createPullRequest).not.toHaveBeenCalled();
 type('#head-branch','feature/checks');type('#pr-body',' Regression covered ');
 vi.mocked(api.createPullRequest).mockRejectedValue(new Error('host denied creation'));
 click(button('Create Pull Request'));await vi.waitFor(()=>expect(view.text()).toContain('host denied creation'));
 expect(api.createPullRequest).toHaveBeenCalledWith('Fix checks','Regression covered','feature/checks','main',false);
 press(view.get('[role="dialog"]'),'Escape');expect(view.find('[role="dialog"]')).toBeNull();
});
