// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, fire, press, render, type Mounted } from '../../testing/mount';
import { openRepository, stashEntry, commit, hunks } from '../../testing/git-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
import * as api from '$lib/api';
import { stash } from '$lib/stash/store.svelte';
import { diff } from '$lib/diff/store.svelte';
import Page from './+page.svelte';
let view:Mounted;
const button=(name:string)=>view.all('button').find(b=>b.textContent?.trim()===name)!;
beforeEach(()=>{
 vi.clearAllMocks();stash.clear();control.reset();openRepository();
 vi.mocked(api.stashes).mockResolvedValue([stashEntry(0)]);
 vi.mocked(api.commitDiff).mockImplementation(async id=>commit(id,['notes.md']));
 vi.mocked(api.fileDiff).mockImplementation(async (_id,path)=>hunks(path));
});
afterEach(()=>{view?.destroy();stash.clear();control.reset();});
it('loads stash contents and keeps typed keys out of hunk navigation',async()=>{
 view=render(Page,{});await vi.waitFor(()=>expect(stash.loaded).toBe(true));
 await vi.waitFor(()=>expect(stash.file).not.toBeNull());
 expect(view.text()).toContain('1 entry');expect(view.text()).toContain('notes.md');
 click(button('split'));expect(diff.view).toBe('split');click(button('unified'));expect(diff.view).toBe('unified');
 expect(press(window,'j').defaultPrevented).toBe(true);expect(press(window,'k').defaultPrevented).toBe(true);
 const input=view.get('[aria-label="Stash message"]') as HTMLInputElement;
 expect(press(input,'j').defaultPrevented).toBe(false);expect(press(window,'j',{ctrlKey:true}).defaultPrevented).toBe(false);
 input.value='WIP tests';fire(input,'input');click(button('include untracked'));click(button('Stash'));
 await vi.waitFor(()=>expect(api.stashPush).toHaveBeenCalledWith('WIP tests',true));
});
it('shows read errors and recovers to an empty stash',async()=>{
 vi.mocked(api.stashes).mockRejectedValueOnce(new Error('stash unavailable'));view=render(Page,{});
 await vi.waitFor(()=>expect(view.text()).toContain('stash unavailable'));
 vi.mocked(api.stashes).mockResolvedValue([]);click(button('Refresh'));
 await vi.waitFor(()=>expect(view.text()).toContain('0 entries'));expect(view.text()).not.toContain('stash unavailable');
});
it('does not read stash data without a repository',()=>{
 control.setInfo(null);view=render(Page,{});expect(view.text()).toContain('No repository open');expect(api.stashes).not.toHaveBeenCalled();
});
