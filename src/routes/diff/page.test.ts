// SPDX-License-Identifier: GPL-3.0-or-later
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { click, press, render, type Mounted } from '../../testing/mount';
import { commit, hunks, openRepository } from '../../testing/git-fixtures';
import { control } from '../../testing/repo-store.svelte';
vi.mock('$lib/api');
vi.mock('$lib/repo.svelte', async () => await import('../../testing/repo-store.svelte'));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/diff?commit=abc') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn() }));
import * as api from '$lib/api';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { diff } from '$lib/diff/store.svelte';
import Page from './+page.svelte';
let view: Mounted;
const button=(name:string)=>view.all('button').find(b=>b.textContent?.trim()===name)!;
beforeEach(()=>{
 vi.clearAllMocks(); diff.clear(); control.reset(); openRepository();
 (page as {url:URL}).url=new URL('http://localhost/diff?commit=abc');
 vi.mocked(api.commitDiff).mockImplementation(async id=>commit(id,['a.txt','b.txt']));
 vi.mocked(api.fileDiff).mockImplementation(async (_id,path)=>hunks(path));
});
afterEach(()=>{view?.destroy();diff.clear();control.reset();});
it('loads the URL commit once and follows file navigation and view changes',async()=>{
 view=render(Page,{}); await vi.waitFor(()=>expect(diff.file?.path).toBe('a.txt'));
 expect(api.commitDiff).toHaveBeenCalledTimes(1); expect(api.commitDiff).toHaveBeenCalledWith('abc');
 expect(view.text()).toContain('2 files · +2 −2'); expect((button('Prev file') as HTMLButtonElement).disabled).toBe(true);
 click(button('Next file')); await vi.waitFor(()=>expect(diff.file?.path).toBe('b.txt'));
 expect(view.text()).toContain('2 of 2'); expect((button('Next file') as HTMLButtonElement).disabled).toBe(true);
 click(button('Prev file')); await vi.waitFor(()=>expect(diff.file?.path).toBe('a.txt'));
 click(button('split')); expect(diff.view).toBe('split'); click(button('unified')); expect(diff.view).toBe('unified');
 expect(press(window,'j').defaultPrevented).toBe(true); expect(press(window,'k').defaultPrevented).toBe(true);
 press(window,'Escape',{ctrlKey:true}); expect(goto).not.toHaveBeenCalled();
 press(window,'Escape'); expect(goto).toHaveBeenCalledWith('/');
});
it('keeps editing keys inside input fields and returns through the Graph button',()=>{
 (page as {url:URL}).url=new URL('http://localhost/diff'); view=render(Page,{});
 expect(view.text()).toContain('No commit chosen'); expect(api.commitDiff).not.toHaveBeenCalled();
 const input=document.createElement('input');view.target.appendChild(input);press(input,'Escape');expect(goto).not.toHaveBeenCalled();
 click(button('← Graph'));expect(goto).toHaveBeenCalledWith('/');
});
it('shows read failures without claiming the commit loaded',async()=>{
 vi.mocked(api.commitDiff).mockRejectedValueOnce(new Error('unknown commit'));view=render(Page,{});
 await vi.waitFor(()=>expect(view.text()).toContain('unknown commit'));expect(diff.commit).toBeNull();
});
it('waits for a repository before reading a deep-linked commit',()=>{
 control.setInfo(null);view=render(Page,{});expect(view.text()).toContain('No repository open');expect(api.commitDiff).not.toHaveBeenCalled();
});
