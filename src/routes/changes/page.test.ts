// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, fire, render, type Mounted } from '../../testing/mount';
import { openRepository, hunks } from '../../testing/git-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
import * as api from '$lib/api';
import { changes } from '$lib/changes/store.svelte';
import Page from './+page.svelte';
let view:Mounted;
const button=(name:string)=>view.all('button').find(b=>b.textContent?.trim()===name)!;
beforeEach(()=>{
 vi.clearAllMocks();changes.clear();control.reset();openRepository();
 vi.mocked(api.workingCopy).mockResolvedValue({staged:[],unstaged:[],conflicted:[]});
 vi.mocked(api.signing).mockRejectedValue(new Error('signing not configured'));
 vi.mocked(api.workingDiff).mockImplementation(async path=>hunks(path));
});
afterEach(()=>{view?.destroy();changes.clear();control.reset();});
it('shows a clean repository and disables an empty commit',async()=>{
 view=render(Page,{});await vi.waitFor(()=>expect(changes.loaded).toBe(true));
 expect(view.text()).toContain('Nothing to commit');expect((button('Commit') as HTMLButtonElement).disabled).toBe(true);
});
it('requires a subject before committing the staged file and reports write failures',async()=>{
 vi.mocked(api.workingCopy).mockResolvedValue({staged:[{path:'a.txt',status:'modified'}],unstaged:[],conflicted:[]});
 view=render(Page,{});await vi.waitFor(()=>expect(changes.loaded).toBe(true));
 expect((button('Commit 1 file') as HTMLButtonElement).disabled).toBe(true);
 const input=view.get('input') as HTMLInputElement;input.value='Fix tests';fire(input,'input');
 vi.mocked(api.commit).mockRejectedValue(new Error('hook rejected commit'));
 click(button('Commit 1 file'));await vi.waitFor(()=>expect(view.text()).toContain('hook rejected commit'));
 expect(api.commit).toHaveBeenCalledWith('Fix tests','',false);
});
it('tells the user why conflicts prevent committing',async()=>{
 vi.mocked(api.workingCopy).mockResolvedValue({staged:[],unstaged:[],conflicted:[{path:'a.txt',status:'modified'}]});
 view=render(Page,{});await vi.waitFor(()=>expect(changes.loaded).toBe(true));
 expect(view.text()).toContain('Resolve the conflicts before committing');expect((button('Commit') as HTMLButtonElement).disabled).toBe(true);
});
it('reports a read error and recovers after refresh',async()=>{
 vi.mocked(api.workingCopy).mockRejectedValueOnce(new Error('index locked'));view=render(Page,{});
 await vi.waitFor(()=>expect(view.text()).toContain('index locked'));click(button('Refresh'));
 await vi.waitFor(()=>expect(view.text()).toContain('Nothing to commit'));
});
